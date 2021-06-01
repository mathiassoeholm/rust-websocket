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
        for byte in bytes {
            self.parse_byte(byte);
        }
    }

    fn parse_byte(&mut self, byte: u8) {
        match self.state {
            ParserState::FirstByte => self.parse_first_byte(byte),
            ParserState::PayloadLength => self.parse_payload_length(byte),
            ParserState::ExtendedPayloadLength => todo!(),
            ParserState::MaskingKey => self.parse_masking_key(byte),
            ParserState::Payload => todo!(),
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
        let mut current_frame = self.unfinished_frame.as_mut().unwrap();

        current_frame.is_masked = byte & 0b10000000 == 0b10000000;

        let payload_length = byte & 0b01111111;
        current_frame.payload_length_type = Some(PayloadLengthType::from_number(payload_length));
        current_frame.payload_length = Some(payload_length as u64);

        self.state = ParserState::MaskingKey;
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
                    let mut current_frame = self.unfinished_frame.as_mut().unwrap();
                    current_frame.masking_key = Some([0; 4]);
                    current_frame
                        .masking_key
                        .as_mut()
                        .unwrap()
                        .copy_from_slice(&byte_buffer[..4]);

                    if let Some(frame_receiver) = &mut self.frame_receiver {
                        frame_receiver.receive(DataFrame {
                            fin: current_frame.fin,
                            opcode: current_frame.opcode,

                            // TODO - Figure out if this also clones the entire vector
                            payload_bytes: current_frame.payload_bytes.clone(),
                        });
                    };

                    self.state = ParserState::FirstByte;
                }
            }
        }
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
        let pong_frame_1 = vec![0b10001010, 0b00000000];
        let pong_frame_2 = vec![
            0b10001010, 0b10000000, /* Masking key: */ 0b10101010, 0b10101010, 0b10101010,
            0b10101010,
        ];

        let mut frame_parser = FrameParser::new();
        let mut frame_receiver = TestFrameReceiver::new();
        frame_parser.set_frame_receiver(&mut frame_receiver);

        frame_parser.parse_bytes(pong_frame_1);
        frame_parser.parse_bytes(pong_frame_2);

        assert_eq!(
            frame_receiver.received_frames,
            vec![
                DataFrame {
                    fin: true,
                    opcode: Opcode::Pong,
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
}
