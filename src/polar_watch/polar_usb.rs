extern crate rusb;
pub mod polar_error;

pub type Device = rusb::Device<rusb::Context>;
pub type DeviceHandle = rusb::DeviceHandle<rusb::Context>;

#[allow(unused_imports)]
use log::{debug, info};
use std::convert::TryFrom;

pub fn tail_bits(value: usize) -> u8 {
    u8::try_from(value % 256).unwrap()
}

// Interface to interact with Polar watches through a USB connection
pub struct PolarUsb {
    handle: DeviceHandle,
}
impl PolarUsb {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    pub const PACKET_SIZE: usize = 64;

    pub fn new(device: rusb::Device<rusb::Context>) -> Result<PolarUsb, rusb::Error> {
        let mut handle = device.open()?;

        // Try to detach kernel driver if it is active. Be careful because operation might not be
        // supported, and in that case don't do anything
        debug!("Trying to detach kernel driver");
        match handle.kernel_driver_active(0) {
            Ok(_) => match handle.detach_kernel_driver(0) {
                // Err(rusb::Error::NotSupported) => Ok(()),
                Err(rusb::Error::NotFound) => Ok(()),
                other => other,
            },
            Err(rusb::Error::NotSupported) => Ok(()),
            Err(other) => Err(other),
        }?;

        debug!("Claiming usb interface");
        handle.claim_interface(0)?;

        Ok(PolarUsb { handle })
    }

    // TODO: Remove one of the request types
    pub fn simple_request(&mut self, data: &[u8]) -> Result<Vec<u8>, polar_error::PolarError> {
        let mut packet = vec![tail_bits(data.len()), 0x0];
        packet.extend_from_slice(data);
        packet.push(0);

        self.request(packet.as_slice())
    }

    pub fn request(&mut self, data: &[u8]) -> Result<Vec<u8>, polar_error::PolarError> {
        debug!("REQUEST {:?}", data);

        let chunk_size = PolarUsb::PACKET_SIZE - 3;

        let packets: Vec<&[u8]> = data.chunks(chunk_size).collect();

        for packet_id in 0..packets.len() {
            let has_more_packets = packet_id != packets.len() - 1;

            self.send_packet(packets[packet_id], packet_id, has_more_packets)?;

            if has_more_packets {
                self.usb_read()?;
            }
        }

        let answer = self.read()?;
        debug!("ANSWER: {:?}", answer);

        Ok(answer)
    }

    fn send_packet(
        &mut self,
        data: &[u8],
        packet_id: usize,
        has_more_packets: bool,
    ) -> Result<usize, polar_error::PolarError> {
        debug!("SEND_PACKET {} {:?}", data.len(), data);

        // Each packet must be exactly full, except for the last one
        assert!(!has_more_packets || data.len() == PolarUsb::PACKET_SIZE - 3);

        // This makes no sense, but that's how it works
        let data_size = if has_more_packets {
            data.len() + 1
        } else {
            data.len() + 1
        };

        let mut packet = vec![0x1, tail_bits(data_size) << 2, tail_bits(packet_id)];
        packet.extend_from_slice(data);

        if has_more_packets {
            packet[1] = packet[1] | 0x01;
        }

        let bytes_written = self.usb_write(packet)?;
        Ok(bytes_written)
    }

    fn read(&mut self) -> Result<Vec<u8>, polar_error::PolarError> {
        let mut packet_id: u8 = 0;
        let mut initial_packet = true;
        let mut data = Vec::new();

        loop {
            let packet = self.usb_read()?;
            debug!("PACKET: {:?}", packet);

            if packet[0] != 0x11 {
                return Err(PolarUsb::proccess_error(packet));
            }

            let mut start: usize = 3;
            let mut size: usize = usize::from(packet[1] >> 2);
            let has_more = (packet[1] & 0x01) != 0;
            let is_notification = (packet[1] & 0x2) != 0;

            if is_notification {
                PolarUsb::proccess_notification(packet);
                continue;
            }

            if initial_packet {
                let is_error = packet[3] != 0;

                if is_error {
                    return Err(PolarUsb::proccess_error(packet));
                }

                size = std::cmp::max(size, 2) - 2;
                start += 2;
            }

            assert!(packet[2] == packet_id);

            if !has_more {
                // Skip trailing 0x0
                let slice_end = if size == 0 { start } else { start + size - 1 };
                data.extend_from_slice(&packet[start..slice_end]);
                return Ok(data);
            } else {
                let slice_end = if size == 0 { start } else { start + size - 1 };
                data.extend_from_slice(&packet[start..slice_end]);

                // Send ack and get the next part
                let ack = [1, 1 << 2 | 0x1, packet_id].to_vec();

                self.usb_write(ack)?;

                if packet_id == 0xff {
                    packet_id = 0;
                } else {
                    packet_id += 1;
                }
            }

            initial_packet = false;
        }
    }

    fn usb_read(&mut self) -> Result<Vec<u8>, polar_error::PolarError> {
        let mut data: [u8; PolarUsb::PACKET_SIZE] = [0; 64];

        self.handle.read_interrupt(
            1 | rusb::constants::LIBUSB_ENDPOINT_IN,
            &mut data,
            PolarUsb::TIMEOUT,
        )?;

        Ok(data.to_vec())
    }

    fn usb_write(&mut self, mut data: Vec<u8>) -> Result<usize, rusb::Error> {
        assert!(data.len() <= PolarUsb::PACKET_SIZE);

        if data.len() < PolarUsb::PACKET_SIZE {
            data.resize(PolarUsb::PACKET_SIZE, 0);
        }

        self.handle.write_interrupt(
            1 | rusb::constants::LIBUSB_ENDPOINT_OUT,
            &data,
            PolarUsb::TIMEOUT,
        )
    }

    fn proccess_notification(data: Vec<u8>) {
        match data[3] {
            10 => info!("Notification received: push notification settings"),
            3 => info!("Notification received: battery status: {}%", data[5]),
            2 => info!("Notification received: device is idling"),
            _ => info!(
                "Notification received: unknown type {} ({:?})",
                data[3], data
            ),
        };
    }

    fn proccess_error(data: Vec<u8>) -> polar_error::PolarError {
        let error = data[3];

        assert!(error != 0);

        match error {
            1 => polar_error::PolarError::new("Error: rebooting"),
            2 => polar_error::PolarError::new("Error: try again"),
            100 => polar_error::PolarError::new("Error: unidentified host error"),
            101 => polar_error::PolarError::new("Error: invalid command"),
            102 => polar_error::PolarError::new("Error: invalid parameter"),
            103 => polar_error::PolarError::new("Error: no such file or directory"),
            104 => polar_error::PolarError::new("Error: directory exists"),
            105 => polar_error::PolarError::new("Error: file exists"),
            106 => polar_error::PolarError::new("Error: operation not permitted"),
            107 => polar_error::PolarError::new("Error: no such user"),
            108 => polar_error::PolarError::new("Error: timeout"),
            200 => polar_error::PolarError::new("Error: unidentified device error"),
            201 => polar_error::PolarError::new("Error: not implemented"),
            202 => polar_error::PolarError::new("Error: system busy"),
            203 => polar_error::PolarError::new("Error: invalid content"),
            204 => polar_error::PolarError::new("Error: checksum failure"),
            205 => polar_error::PolarError::new("Error: disk full"),
            206 => polar_error::PolarError::new("Error: prerequisite not found"),
            207 => polar_error::PolarError::new("Error: insufficient buffer"),
            208 => polar_error::PolarError::new("Error: wait for idling"),
            _ => polar_error::PolarError::new("Error: unknown error #{packet[3]}"),
        }
    }
}
