#[derive(PartialEq, Debug, Clone, Copy)]
enum Opcode {
    Ping,
    Pong,
    Unknown,
}

impl Opcode {
    fn from_u8(value: u8) -> Opcode {
        match value {
            0x09 => Opcode::Ping,
            0x0A => Opcode::Pong,
            _ => Opcode::Unknown,
        }
    }
}

#[derive(PartialEq, Debug)]
enum PayloadLengthType {
    Normal,
    Extended,
    LongExtended,
}

impl PayloadLengthType {
    fn from_number(value: u8) -> PayloadLengthType {
        PayloadLengthType::Normal
    }
}

#[derive(PartialEq, Debug)]
pub struct DataFrame {
    fin: bool,
    opcode: Opcode,
    payload_bytes: Option<Vec<u8>>,
}

struct UnfinishedDataFrame {
    fin: bool,
    opcode: Opcode,
    payload_length_type: Option<PayloadLengthType>,
    payload_length: Option<u64>,
    is_masked: bool,
    masking_key: Option<[u8; 4]>,
    payload_bytes: Option<Vec<u8>>,
}

pub trait DataFrameReceiver {
    fn receive(&mut self, frame: DataFrame);
}

#[derive(PartialEq, Debug)]
enum ParserState {
    // We are waiting for the first byte of the frame.
    FirstByte,

    // We are waiting for the payload length byte of the frame.
    PayloadLength,

    // Optional state that happens if frame.payload_length > 126. Here, we wait for all the bytes to finish the extended payload length.
    ExtendedPayloadLength,

    // We are reading the bytes of the masking key.
    MaskingKey,

    // We are reading the bytes of the payload.
    Payload,
}

pub struct FrameParser<'a> {
    unfinished_frame: Option<UnfinishedDataFrame>,
    state: ParserState,

    // A buffer used when reading the bytes
    byte_buffer: Option<Vec<u8>>,

    frame_receiver: Option<&'a mut dyn DataFrameReceiver>,
}

static PING_FRAME: [u8; 2] = [0b10001001, 0b00000000];

impl<'a> FrameParser<'a> {
    pub fn new() -> FrameParser<'a> {
        FrameParser {
            unfinished_frame: None,
            byte_buffer: None,
            frame_receiver: None,
            state: ParserState::FirstByte,
        }
    }

    fn set_frame_receiver(&mut self, frame_receiver: &'a mut dyn DataFrameReceiver) {
        self.frame_receiver = Some(frame_receiver);
    }

    pub fn receive(&mut self, bytes: Vec<u8>) {
        self.parse_bytes(bytes);
    }

    fn parse_bytes(&mut self, bytes: Vec<u8>) {
        for (index, byte) in bytes.iter().enumerate() {
            self.parse_byte(*byte, &bytes[index..]);
        }
    }

    fn parse_byte(&mut self, byte: u8, remaining_bytes: &[u8]) {
        match self.state {
            ParserState::FirstByte => self.parse_first_byte(byte),
            ParserState::PayloadLength => self.parse_payload_length(byte),
            ParserState::ExtendedPayloadLength => todo!(),
            ParserState::MaskingKey => self.parse_masking_key(byte),
            ParserState::Payload => self.parse_payload(remaining_bytes),
        };
    }

    fn parse_first_byte(&mut self, byte: u8) {
        self.unfinished_frame = Some(UnfinishedDataFrame {
            fin: byte & 0b10000000 == 0b10000000,
            opcode: Opcode::from_u8(byte & 0b00001111),
            payload_length_type: None,
            payload_length: None,
            is_masked: false,
            masking_key: None,
            payload_bytes: None,
        });

        self.state = ParserState::PayloadLength;
    }

    fn parse_payload_length(&mut self, byte: u8) {
        let mut unfinished_frame = self.unfinished_frame.as_mut().unwrap();

        unfinished_frame.is_masked = byte & 0b10000000 == 0b10000000;

        let payload_length = byte & 0b01111111;
        unfinished_frame.payload_length_type = Some(PayloadLengthType::from_number(payload_length));
        unfinished_frame.payload_length = Some(payload_length as u64);

        if unfinished_frame.is_masked {
            self.state = ParserState::MaskingKey;
        } else if unfinished_frame.payload_length > Some(0) {
            self.state = ParserState::Payload;
        } else {
            self.finish_frame();
        };
    }

    fn parse_masking_key(&mut self, byte: u8) {
        match &mut self.byte_buffer {
            None => {
                // This is the first byte of the masking key
                let mut key_buffer: Vec<u8> = Vec::with_capacity(4);
                key_buffer.push(byte);

                self.byte_buffer = Some(key_buffer);
            }

            Some(byte_buffer) => {
                byte_buffer.push(byte);

                if byte_buffer.len() == 4 {
                    let mut unfinished_frame = self.unfinished_frame.as_mut().unwrap();
                    unfinished_frame.masking_key = Some([0; 4]);
                    unfinished_frame
                        .masking_key
                        .as_mut()
                        .unwrap()
                        .copy_from_slice(&byte_buffer[..4]);

                    if unfinished_frame.payload_length > Some(0) {
                        self.state = ParserState::Payload;
                    } else {
                        self.finish_frame();
                    };
                }
            }
        }
    }

    fn parse_payload(&mut self, remaining_bytes: &[u8]) {
        let unfinished_frame = self.unfinished_frame.as_mut().unwrap();
        let payload_length = unfinished_frame.payload_length.unwrap();

        if payload_length == 0 {
            self.finish_frame();
            return;
        }

        let payload = &remaining_bytes[..(payload_length as usize)];
        unfinished_frame.payload_bytes = Some(payload.to_vec());
        self.finish_frame();
    }

    pub fn finish_frame(&mut self) {
        if let Some(frame_receiver) = &mut self.frame_receiver {
            if let Some(finished_frame) = &self.unfinished_frame {
                frame_receiver.receive(DataFrame {
                    fin: finished_frame.fin,
                    opcode: finished_frame.opcode,

                    // TODO - Figure out if this also clones the entire vector
                    payload_bytes: finished_frame.payload_bytes.clone(),
                });
            }
        };

        self.unfinished_frame = None;
        self.state = ParserState::FirstByte;
    }

    pub fn create_ping_frame() -> &'static [u8] {
        return &PING_FRAME;
    }
}

#[cfg(test)]
mod test {

    use super::*;
    struct TestFrameReceiver {
        received_frames: Vec<DataFrame>,
    }

    impl TestFrameReceiver {
        fn new() -> TestFrameReceiver {
            TestFrameReceiver {
                received_frames: Vec::new(),
            }
        }
    }

    impl<'a> DataFrameReceiver for TestFrameReceiver {
        fn receive(&mut self, frame: DataFrame) {
            self.received_frames.push(frame)
        }
    }

    #[test]
    fn it_should_parse_frame() {
        let pong_frame = vec![
            0b10001010, 0b10000000, /* Masking key: */ 0b10101010, 0b10101010, 0b10101010,
            0b10101010,
        ];

        let mut frame_receiver = TestFrameReceiver::new();
        let mut frame_parser = FrameParser::new();
        frame_parser.set_frame_receiver(&mut frame_receiver);

        frame_parser.parse_bytes(pong_frame);

        assert_eq!(
            frame_receiver.received_frames,
            vec![DataFrame {
                fin: true,
                opcode: Opcode::Pong,
                payload_bytes: None,
            }]
        );
    }

    #[test]
    fn it_should_parse_partial_frames() {
        let pong_bytes_1 = vec![0b10001010, 0b10000000, /* Masking key: */ 0b10101010];
        let pong_bytes_2 = vec![
            /* Remaining mask-keys: */ 0b10101010, 0b10101010, 0b10101010,
        ];

        let mut frame_receiver = TestFrameReceiver::new();
        let mut frame_parser = FrameParser::new();
        frame_parser.set_frame_receiver(&mut frame_receiver);

        frame_parser.parse_bytes(pong_bytes_1);
        frame_parser.parse_bytes(pong_bytes_2);

        assert_eq!(
            frame_receiver.received_frames,
            vec![DataFrame {
                fin: true,
                opcode: Opcode::Pong,
                payload_bytes: None,
            }]
        );
    }

    #[test]
    fn it_supports_multiple_frames() {
        let ping_frame = vec![0b10001001, 0b00000000];
        let pong_frame = vec![0b10001010, 0b00000000];

        let mut frame_parser = FrameParser::new();
        let mut frame_receiver = TestFrameReceiver::new();
        frame_parser.set_frame_receiver(&mut frame_receiver);

        frame_parser.parse_bytes(ping_frame);
        frame_parser.parse_bytes(pong_frame);

        assert_eq!(
            frame_receiver.received_frames,
            vec![
                DataFrame {
                    fin: true,
                    opcode: Opcode::Ping,
                    payload_bytes: None,
                },
                DataFrame {
                    fin: true,
                    opcode: Opcode::Pong,
                    payload_bytes: None,
                }
            ]
        );
    }

    #[test]
    fn it_parses_short_byte_payload() {
        let payload = vec![0b00000001, 0b00000010];
        let frame_with_short_payload = [vec![0b10000001, 0b00000010], payload.clone()].concat();

        let mut frame_parser = FrameParser::new();
        let mut frame_receiver = TestFrameReceiver::new();
        frame_parser.set_frame_receiver(&mut frame_receiver);

        frame_parser.parse_bytes(frame_with_short_payload);
        assert_eq!(
            frame_receiver.received_frames,
            vec![DataFrame {
                fin: true,
                opcode: Opcode::Unknown,
                payload_bytes: Some(payload),
            }],
        );
    }

    #[test]
    fn it_parses_masked_payload() {
        let mask = vec![0b10110101, 0b00000000, 0b11111111, 0b10110000];
        let payload = vec![0b00000001, 0b00000010, 0b00010111, 0b10110011, 0b00000001];
        let masked_payload = vec![
            payload[0] ^ mask[0],
            payload[1] ^ mask[1],
            payload[2] ^ mask[2],
            payload[3] ^ mask[3],
            payload[4] ^ mask[0],
        ];
        let frame_with_masked_payload = [
            vec![0b10000001, 0b10000101],
            mask.clone(),
            masked_payload.clone(),
        ]
        .concat();

        let mut frame_parser = FrameParser::new();
        let mut frame_receiver = TestFrameReceiver::new();
        frame_parser.set_frame_receiver(&mut frame_receiver);

        frame_parser.parse_bytes(frame_with_masked_payload);

        assert_eq!(
            frame_receiver.received_frames,
            vec![DataFrame {
                fin: true,
                opcode: Opcode::Unknown,
                payload_bytes: Some(payload),
            }],
        );
    }
}
