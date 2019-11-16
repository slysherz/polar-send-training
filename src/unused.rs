

    fn request_put_initial(
        &mut self,
        data: Vec<u8>,
        dest: Vec<u8>,
        data_left: usize,
    ) -> Result<bool, polar_error::PolarError> {
        let total_len = tail_bits(data.len() + dest.len());
        let dest_len = tail_bits(dest.len());

        let mut packet: Vec<u8> = vec![
            0x1,
            (total_len + 6) << 2,
            0,
            dest_len + 4,
            0x0,
            0x8,
            0x1,
            0x12,
            dest_len,
        ];

        packet.extend_from_slice(dest.as_slice());
        packet.extend_from_slice(data.as_slice());

        // more packets
        if data_left > 0 {
            packet[1] = packet[1] | 0x01;
        }

        // print "#{data.size} left #{data_left} #{data.bytes}\n"
        self.usb_write(packet)?;
        self.read_status()
    }

    fn request_put_next(
        &mut self,
        data: Vec<u8>,
        packet_num: u8,
        data_left: usize,
    ) -> Result<bool, polar_error::PolarError> {
        let data_len = tail_bits(data.len());

        let mut packet: Vec<u8> = vec![0x1, (data_len + 1) << 2, packet_num];

        packet.extend_from_slice(data.as_slice());

        // more packets
        if data_left > 0 {
            packet[1] = packet[1] | 0x01;
        }

        // print "#{data.size} left #{data_left} #{data.bytes}\n"
        self.usb_write(packet);
        self.read_status()
    }



    fn send_packet0(
        &mut self,
        data: &[u8],
        packet_id: usize,
        size_left: usize,
    ) -> Result<Vec<u8>, polar_error::PolarError> {
        let size_after = size_left - data.len();
        let has_more_packets = size_after > 0;

        // Each packet must be exactly full, except for the last one
        assert!(size_after == 0 || data.len() == PolarUsb::PACKET_SIZE - 3);

        let size_remaining = if has_more_packets {
            size_left
        } else {
            size_left + 1
        };


        let mut packet = vec![0x1, tail_bits(size_remaining) << 2, tail_bits(packet_id)];
        packet.extend_from_slice(data);

        if has_more_packets {
            packet[1] = packet[1] | 0x01;
        }

        println!(
            "DATA:
    left {}
    total {}
    {:?}",
            size_left,
            data.len(),
            packet
        );

        Ok([].to_vec())
    }



    fn quick_request(&mut self, data: Vec<u8>) -> Result<Vec<u8>, polar_error::PolarError> {
        let size = u8::try_from(data.len()).expect("Payload is too big");

        let mut packet = vec![size, 0x0];
        packet.append(&mut data.clone());

        self.send_packet(&packet[..], 0, data.len() + 2)
    }



    fn read_status(&mut self) -> Result<bool, polar_error::PolarError> {
        let packet = self.usb_read()?;
        let is_command_end = (packet[1] & 0x10) != 0;

        Ok(is_command_end)
    }