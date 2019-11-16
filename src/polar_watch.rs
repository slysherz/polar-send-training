pub extern crate polar_prost as polar;
mod polar_usb;

pub use polar::{encode, Message};
pub use polar_usb::polar_error::PolarError;

use log::{debug, info};
use polar_usb::{Device, PolarUsb};
use rusb::UsbContext;

pub struct PolarWatch {
    handle: PolarUsb,
}

impl PolarWatch {
    pub fn new(handle: PolarUsb) -> PolarWatch {
        PolarWatch { handle: handle }
    }

    fn find_compatible_devices(context: &mut rusb::Context) -> Result<Vec<Device>, PolarError> {
        let mut devices = Vec::new();

        for device in context.devices()?.iter() {
            let descriptor = device.device_descriptor()?;
            if descriptor.vendor_id() == 0x0da4 && descriptor.product_id() == 0x0008 {
                devices.push(device)
            }
        }

        info!("Found {} compatible devices", devices.len());
        Ok(devices)
    }

    pub fn find_one(context: &mut rusb::Context) -> Result<PolarWatch, PolarError> {
        let mut devices = PolarWatch::find_compatible_devices(context)?;
        match devices.pop() {
            Some(device) => Ok(PolarWatch::new(PolarUsb::new(device)?)),
            _ => Err(PolarError::new("Watch not found")),
        }
    }

    pub fn find_all(context: &mut rusb::Context) -> Result<Vec<PolarWatch>, PolarError> {
        let mut watches = Vec::new();

        for device in PolarWatch::find_compatible_devices(context)? {
            let watch = PolarWatch::new(PolarUsb::new(device)?);
            watches.push(watch);
        }

        Ok(watches)
    }

    pub fn send_file<S>(&mut self, path: S, data: &[u8]) -> Result<(), PolarError>
    where
        S: Into<String>,
    {
        let path: String = path.into();

        info!("Uploading to {} data: {:?} ", path, data);
        let path_len = polar_usb::tail_bits(path.len());

        let mut packet: Vec<u8> = vec![path_len + 4, 0x0, 0x8, 0x1, 0x12, path_len];
        packet.extend_from_slice(path.as_bytes());
        packet.extend_from_slice(data);
        packet.push(0);

        self.handle.request(packet.as_slice())?;

        // Check if the file is there
        let file = self.get_file(path)?;

        if file != data {
            return Err(PolarError::new(format!(
                "Content on the watch doesn't match the one sent.    sent {:?}    got {:?}",
                data, file
            )));
        }

        Ok(())
    }

    pub fn get_file<S>(&mut self, path: S) -> Result<Vec<u8>, PolarError>
    where
        S: Into<String>,
    {
        let path: String = path.into();

        info!("Downloading {}", path);
        let request = encode(polar::protocol::PbPFtpOperation {
            command: 0,
            path: path.clone(),
        })
        .unwrap();

        let mut answer = self.handle.simple_request(request.as_slice())?;
        debug!("FILE {:?}", answer);

        assert!(answer.pop().unwrap() == 0);
        Ok(answer)
    }

    pub fn delete_file<S>(&mut self, path: S) -> Result<(), PolarError>
    where
        S: Into<String>,
    {
        let path: String = path.into();

        info!("Deleting {}", path);
        let request = encode(polar::protocol::PbPFtpOperation {
            command: 3,
            path: path,
        })
        .unwrap();

        self.handle.simple_request(request.as_slice())?;

        Ok(())
    }

    pub fn dir<S>(&mut self, path: S) -> Result<Vec<String>, PolarError>
    where
        S: Into<String>,
    {
        let mut path: String = path.into();

        if !path.ends_with("/") {
            path = path + "/";
        }

        let answer = self.get_file(path)?;
        let mut result = vec![];
        for entry in polar::protocol::PbPFtpDirectory::decode(answer)
            .unwrap()
            .entries
        {
            result.push(entry.name);
        }

        Ok(result)
    }

    pub fn mkdir<S>(&mut self, path: S) -> Result<(), PolarError>
    where
        S: Into<String>,
    {
        let mut path: String = path.into();

        if !path.ends_with("/") {
            path = path + "/";
        }

        let request = encode(polar::protocol::PbPFtpOperation {
            command: 1,
            path: path.clone(),
        })
        .unwrap();

        self.handle.simple_request(request.as_slice())?;

        Ok(())
    }

    pub fn recursive_delete<S>(&mut self, path: S) -> Result<(), PolarError>
    where
        S: Into<String>,
    {
        let mut path: String = path.into();

        if !path.ends_with("/") {
            path = path + "/";
        }

        for entry in self.dir(path.clone())? {
            if entry.ends_with("/") {
                self.recursive_delete(path.clone() + &entry)?;
            } else {
                self.delete_file(path.clone() + &entry)?;
            }
        }

        Ok(())
    }

    pub fn delete_all_favorites(&mut self) -> Result<(), PolarError> {
        info!("Deleting old favorite files");
        let favorites_path = "/U/0/FAV";
        self.recursive_delete(favorites_path)
    }
}
