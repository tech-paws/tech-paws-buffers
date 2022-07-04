impl IntoVMBuffers for MyEnum {
    fn read_from_buffers(bytes_reader: &mut BytesReader) -> Self {
        let value = bytes_reader.read_u32();

        match value {
            1 => return MyEnum::Idle,
            2 => return MyEnum::Move {
                x: bytes_reader.read_f64(),
                y: bytes_reader.read_f64(),
            },
            3 => return MyEnum::Update(
                bytes_reader.read_f64(),
                bytes_reader.read_f64(),
                String::read_from_buffers(bytes_reader),
            ),
            _ => panic!("Unsupported enum value: {}", value),
        }
    }

    fn write_to_buffers(&self, bytes_writer: &mut BytesWriter) {
        match self {
            MyEnum::Idle => {
                bytes_writer.write_u32(1);
            },
            MyEnum::Move {
                x,
                y,
            } => {
                bytes_writer.write_u32(2);
                bytes_writer.write_f64(*x);
                bytes_writer.write_f64(*y);
            },
            MyEnum::Update(
                v0,
                v1,
                v2,
            ) => {
                bytes_writer.write_u32(3);
                bytes_writer.write_f64(*v0);
                bytes_writer.write_f64(*v1);
                v2.write_to_buffers(bytes_writer);
            },
        }
    }

    fn skip_in_buffers(bytes_reader: &mut BytesReader, count: u64) {
        for _ in 0..count {
            let value = bytes_reader.read_u32();

            match value {
                1 => (),
                2 => {
                    bytes_reader.read_f64();
                    bytes_reader.read_f64();
                },
                3 => {
                    bytes_reader.read_f64();
                    bytes_reader.read_f64();
                    String::read_from_buffers(bytes_reader);
                },
                _ => panic!("Unsupported enum value: {}", value),
            }
        }
    }
}