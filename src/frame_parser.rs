use std::cmp::min;
use std::iter::Take;

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
    masking_key: Option<Vec<u8>>,
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
static MASKING_KEY_LENGTH: usize = 4; // bytes

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

    pub fn receive(&mut self, bytes: &Vec<u8>) {
        self.parse_bytes(&bytes);
    }

    fn parse_bytes(&mut self, bytes: &Vec<u8>) {
        // Check the state
        // Allow the state to consume as many bytes as it needs
        let mut bytes_copy = &mut bytes.clone();

        while bytes_copy.len() > 0 {
            match self.state {
                ParserState::FirstByte => self.parse_first_byte(&mut bytes_copy),
                ParserState::PayloadLength => self.parse_payload_length(&mut bytes_copy),
                ParserState::ExtendedPayloadLength => todo!(),
                ParserState::MaskingKey => self.parse_masking_key(&mut bytes_copy),
                ParserState::Payload => self.parse_payload(&mut bytes_copy),
            };
        }
        // self.parse_byte(bytes.iter());
    }

    // fn parse_byte(&mut self, bytes: dyn std::iter::Itertor<u8>) {
    //     match self.state {
    //         ParserState::FirstByte => self.parse_first_byte(byte),
    //         ParserState::PayloadLength => self.parse_payload_length(byte),
    //         ParserState::ExtendedPayloadLength => todo!(),
    //         ParserState::MaskingKey => self.parse_masking_key(remaining_bytes),
    //         ParserState::Payload => self.parse_payload(remaining_bytes),
    //     };
    // }

    fn parse_first_byte(&mut self, bytes: &mut Vec<u8>) {
        let first_byte = consume_one(bytes);
        self.unfinished_frame = Some(UnfinishedDataFrame {
            fin: first_byte & 0b10000000 == 0b10000000,
            opcode: Opcode::from_u8(first_byte & 0b00001111),
            payload_length_type: None,
            payload_length: None,
            is_masked: false,
            masking_key: None,
            payload_bytes: None,
        });

        self.state = ParserState::PayloadLength;
    }

    fn parse_payload_length(&mut self, bytes: &mut Vec<u8>) {
        let byte = consume_one(bytes);
        let mut unfinished_frame = self.unfinished_frame.as_mut().unwrap();

        unfinished_frame.is_masked = byte & 0b10000000 == 0b10000000;

        let payload_length = byte & 0b01111111;
        unfinished_frame.payload_length_type = Some(PayloadLengthType::from_number(payload_length));
        unfinished_frame.payload_length = Some(payload_length as u64);

        if unfinished_frame.payload_length == Some(0) {
            self.finish_frame();
        } else if unfinished_frame.is_masked {
            self.state = ParserState::MaskingKey;
        } else {
            self.state = ParserState::Payload;
        };
    }

    fn parse_masking_key(&mut self, bytes: &mut Vec<u8>) {
        let unfinished_frame = self.unfinished_frame.as_mut().unwrap();
        if unfinished_frame.masking_key == None {
            unfinished_frame.masking_key = Some(Vec::with_capacity(MASKING_KEY_LENGTH))
        };
        let masking_key = unfinished_frame.masking_key.as_mut().unwrap();

        let remaining_masking_key_bytes = MASKING_KEY_LENGTH - masking_key.len();

        // We can't take any more bytes than are available in the incoming bytes vector
        let bytes_to_take = min(remaining_masking_key_bytes, bytes.len());

        let mut masking_key_bytes = consume(bytes, bytes_to_take);
        masking_key.append(&mut masking_key_bytes);

        if masking_key.len() == 4 {
            self.state = ParserState::Payload;
        }
    }

    fn parse_payload(&mut self, bytes: &mut Vec<u8>) {
        let unfinished_frame = self.unfinished_frame.as_mut().unwrap();
        let payload_length = unfinished_frame.payload_length.unwrap();

        if payload_length == 0 {
            self.finish_frame();
            return;
        }

        if unfinished_frame.payload_bytes == None {
            unfinished_frame.payload_bytes = Some(Vec::with_capacity(payload_length as usize));
        }
        let unfinished_frame_payload = unfinished_frame.payload_bytes.as_mut().unwrap();

        // Figure out how many bytes we still need in the payload
        let unfinished_frame_payload_len = unfinished_frame_payload.len();
        let bytes_left_of_payload = payload_length as usize - unfinished_frame_payload_len;

        // We can't take any more bytes than are available in the incoming bytes vector
        let bytes_to_take = min(bytes_left_of_payload, bytes.len());

        let payload_bytes = consume(bytes, bytes_to_take);

        if unfinished_frame.is_masked {
            let masking_key = unfinished_frame.masking_key.as_ref().unwrap();
            unfinished_frame_payload.extend(payload_bytes.iter().enumerate().map(
                |(index, byte)| {
                    let index_in_entire_payload = unfinished_frame_payload_len + index;
                    byte ^ masking_key[index_in_entire_payload % 4]
                },
            ));
        } else {
            unfinished_frame_payload.extend(payload_bytes);
        }

        // Check if bytes contained the rest of the payload
        if bytes_to_take == bytes_left_of_payload {
            self.finish_frame();
        }
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

fn consume_one<T: Copy>(vec: &mut Vec<T>) -> T {
    consume(vec, 1)[0]
}

fn consume<T: Copy>(vec: &mut Vec<T>, amount: usize) -> Vec<T> {
    vec.drain(0..amount).collect()
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

        frame_parser.parse_bytes(&pong_frame);

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

        frame_parser.parse_bytes(&pong_bytes_1);
        frame_parser.parse_bytes(&pong_bytes_2);

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

        frame_parser.parse_bytes(&ping_frame);
        frame_parser.parse_bytes(&pong_frame);

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

        frame_parser.parse_bytes(&frame_with_short_payload);
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

        frame_parser.parse_bytes(&frame_with_masked_payload);

        assert_eq!(
            frame_receiver.received_frames,
            vec![DataFrame {
                fin: true,
                opcode: Opcode::Unknown,
                payload_bytes: Some(payload),
            }],
        );
    }

    // fn it_supports_payload_split_in_multiple_frames
}
