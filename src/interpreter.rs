use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};

use crate::interpreter::Op::{Add, In, JmpNz, JmpZ, Move, Out};

#[derive(Debug, PartialEq)]
enum Op {
    Move { d: isize },
    Add { d: isize },
    Out,
    In,
    JmpZ { addr: usize },
    JmpNz { addr: usize },
}

#[derive(Debug)]
struct LeftBracketInfo {
    line: usize,
    col: usize,
    addr: usize,
}

#[derive(Debug, PartialEq)]
pub enum BuildErrorKind {
    BracketNotMatch,
    BracketNotClosed,
}

#[derive(Debug, PartialEq)]
pub struct BuildError {
    line: usize,
    col: usize,
    kind: BuildErrorKind,
}

#[derive(Debug, PartialEq)]
pub enum RuntimeErrorKind {
    DataOverflow { idx: isize },
    IO { err: String },
}

#[derive(Debug, PartialEq)]
pub struct RuntimeError {
    kind: RuntimeErrorKind,
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            RuntimeErrorKind::DataOverflow { idx } => write!(f, "data overflow, idx = {}", idx),
            RuntimeErrorKind::IO { err } => write!(f, "io err: {}", err),
        }
    }
}

impl Error for RuntimeError {}

#[derive(Debug)]
pub struct Interpreter {
    ops: Vec<Op>,
}

impl Interpreter {
    pub fn build(code: &str) -> Result<Self, BuildError> {
        let bytes = code.as_bytes().iter().map(|c| *c).collect::<Vec<u8>>();
        let mut result = vec![];
        let mut line = 1usize;
        let mut col = 1usize;
        let mut i = 0;
        let mut jmp_stack = vec![];
        while i < bytes.len() {
            let c = bytes[i];
            match c {
                b'<' | b'>' => {
                    let mut delta = if c == b'<' { -1 } else { 1 };
                    while i + 1 < bytes.len() && (bytes[i + 1] == b'<' || bytes[i + 1] == b'>') {
                        delta += if bytes[i + 1] == b'<' { -1 } else { 1 };
                        i += 1;
                    }
                    if delta != 0 {
                        result.push(Move { d: delta });
                    }
                }
                b'+' | b'-' => {
                    let mut delta = if c == b'-' { -1 } else { 1 };
                    while i + 1 < bytes.len() && (bytes[i + 1] == b'-' || bytes[i + 1] == b'+') {
                        delta += if bytes[i + 1] == b'-' { -1 } else { 1 };
                        i += 1;
                    }
                    if delta != 0 {
                        result.push(Add { d: delta });
                    }
                }
                b'.' => {
                    result.push(Out);
                }
                b',' => {
                    result.push(In);
                }
                b'[' => {
                    result.push(JmpZ { addr: 0 });
                    jmp_stack.push(LeftBracketInfo {
                        line,
                        col,
                        addr: result.len(),
                    });
                }
                b']' => match jmp_stack.pop() {
                    Some(info) => {
                        result.push(JmpNz { addr: info.addr });
                        result[info.addr - 1] = JmpZ { addr: result.len() };
                    }
                    None => {
                        return Err(BuildError {
                            line,
                            col,
                            kind: BuildErrorKind::BracketNotMatch,
                        });
                    }
                },
                b'\n' => {
                    line += 1;
                    col = 0;
                }
                _ => {}
            }
            col += 1;
            i += 1;
        }

        if let Some(info) = jmp_stack.pop() {
            return Err(BuildError {
                line: info.line,
                col: info.col,
                kind: BuildErrorKind::BracketNotClosed,
            });
        }

        Ok(Self { ops: result })
    }

    pub fn execute(&self, read: &mut dyn Read, write: &mut dyn Write) -> Result<(), RuntimeError> {
        let mut data = [0u8; 30000];
        let mut d_offset = 0usize; // 0~29999
        let mut i_offset = 0usize;

        while i_offset < self.ops.len() {
            match self.ops[i_offset] {
                Move { d } => {
                    if d < 0 && -d as usize > d_offset || d_offset as isize + d >= 30000 {
                        return Err(RuntimeError {
                            kind: RuntimeErrorKind::DataOverflow {
                                idx: d_offset as isize + d,
                            },
                        });
                    }
                    d_offset = (d_offset as isize + d) as usize;
                }
                Add { d } => data[d_offset] = (data[d_offset] as isize + d) as u8,
                Out => {
                    write
                        .write(&data[d_offset..d_offset + 1])
                        .map_err(|err| RuntimeError {
                            kind: RuntimeErrorKind::IO {
                                err: err.to_string(),
                            },
                        })?;
                }
                In => {
                    read.read_exact(&mut data[d_offset..d_offset + 1])
                        .map_err(|err| RuntimeError {
                            kind: RuntimeErrorKind::IO {
                                err: err.to_string(),
                            },
                        })?;
                }
                JmpZ { addr } => {
                    if data[d_offset] == 0 {
                        i_offset = addr - 1;
                    }
                }
                JmpNz { addr } => {
                    if data[d_offset] != 0 {
                        i_offset = addr - 1;
                    }
                }
            }

            i_offset += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    struct MockInOut {
        data: VecDeque<u8>,
        bad: bool,
    }

    impl MockInOut {
        fn new(d: Vec<u8>) -> Self {
            let data = VecDeque::from(d);
            Self { data, bad: false }
        }

        fn dummy() -> Self {
            Self {
                data: VecDeque::new(),
                bad: false,
            }
        }

        fn bad() -> Self {
            Self {
                data: VecDeque::new(),
                bad: true,
            }
        }
    }

    impl Read for MockInOut {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.bad {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "read"));
            }

            let mut cnt = 0usize;
            for i in 0..buf.len() {
                if let Some(c) = self.data.pop_front() {
                    buf[i] = c;
                    cnt += 1;
                } else {
                    break;
                }
            }
            Ok(cnt)
        }
    }

    impl Write for MockInOut {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.bad {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "write"));
            }

            for c in buf {
                self.data.push_back(*c);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_basic() {
        let code = "<+>-.,[]";
        let interpreter = Interpreter::build(code).unwrap();

        let expected = vec![
            Move { d: -1 },
            Add { d: 1 },
            Move { d: 1 },
            Add { d: -1 },
            Out,
            In,
            JmpZ { addr: 8 },
            JmpNz { addr: 7 },
        ];

        assert_eq!(expected.len(), interpreter.ops.len());
        for (idx, op) in interpreter.ops.iter().enumerate() {
            assert_eq!(expected[idx], *op);
        }
    }

    #[test]
    fn test_fold() {
        let code = "<><<>><+-++--+<>+-";
        let interpreter = Interpreter::build(code).unwrap();

        let expected = vec![Move { d: -1 }, Add { d: 1 }];

        assert_eq!(expected.len(), interpreter.ops.len());
        for (idx, op) in interpreter.ops.iter().enumerate() {
            assert_eq!(expected[idx], *op);
        }
    }

    #[test]
    fn test_not_match() {
        let code = r#"[[
]]]+++"#;

        let err = Interpreter::build(code).unwrap_err();
        assert_eq!(
            BuildError {
                line: 2,
                col: 3,
                kind: BuildErrorKind::BracketNotMatch,
            },
            err
        );
    }

    #[test]
    fn test_not_closed() {
        let code = r#"[[[
]]++"#;
        let err = Interpreter::build(code).unwrap_err();
        assert_eq!(
            BuildError {
                line: 1,
                col: 1,
                kind: BuildErrorKind::BracketNotClosed,
            },
            err
        );
    }

    #[test]
    fn test_input_output() {
        let code = ",>,.<.";
        let inter = Interpreter::build(code).unwrap();
        let mut input = MockInOut::new(vec![b'h', b'i']);
        let mut out = MockInOut::dummy();
        inter.execute(&mut input, &mut out).unwrap();
        assert_eq!(
            "ih".as_bytes(),
            out.data.iter().map(|c| *c).collect::<Vec<u8>>()
        );


        let mut bad_input = MockInOut::bad();
        let mut bad_output = MockInOut::bad();
        let err = inter.execute(&mut bad_input, &mut bad_output).unwrap_err();
        assert_eq!("io err: read", err.to_string());

        let mut input = MockInOut::new(vec![b'h', b'i']);
        let mut bad_output = MockInOut::bad();
        let err = inter.execute(&mut input, &mut bad_output).unwrap_err();
        assert_eq!("io err: write", err.to_string());
    }

    #[test]
    fn test_sample1() {
        let code = r#"
++       Cell c0 = 2
> +++++  Cell c1 = 5

[        Start your loops with your cell pointer on the loop counter (c1 in our case)
< +      Add 1 to c0
> -      Subtract 1 from c1
]        End your loops with the cell pointer on the loop counter

At this point our program has added 5 to 2 leaving 7 in c0 and 0 in c1
but we cannot output this value to the terminal since it is not ASCII encoded

To display the ASCII character "7" we must add 48 to the value 7
We use a loop to compute 48 = 6 * 8

++++ ++++  c1 = 8 and this will be our loop counter again
[
< +++ +++  Add 6 to c0
> -        Subtract 1 from c1
]
< .        Print out c0 which has the value 55 which translates to "7"!
        "#;
        let inter = Interpreter::build(code).unwrap();
        let mut out = MockInOut::dummy();
        inter.execute(&mut MockInOut::dummy(), &mut out).unwrap();

        assert_eq!(1, out.data.len());
        assert_eq!(55, out.data[0]);
    }

    #[test]
    fn test_hello_world() {
        let code = r#"
[ This program prints "Hello World!" and a newline to the screen, its
  length is 106 active command characters. [It is not the shortest.]

  This loop is an "initial comment loop", a simple way of adding a comment
  to a BF program such that you don't have to worry about any command
  characters. Any ".", ",", "+", "-", "<" and ">" characters are simply
  ignored, the "[" and "]" characters just have to be balanced. This
  loop and the commands it contains are ignored because the current cell
  defaults to a value of 0; the 0 value causes this loop to be skipped.
]
++++++++               Set Cell #0 to 8
[
    >++++               Add 4 to Cell #1; this will always set Cell #1 to 4
    [                   as the cell will be cleared by the loop
        >++             Add 2 to Cell #2
        >+++            Add 3 to Cell #3
        >+++            Add 3 to Cell #4
        >+              Add 1 to Cell #5
        <<<<-           Decrement the loop counter in Cell #1
    ]                   Loop until Cell #1 is zero; number of iterations is 4
    >+                  Add 1 to Cell #2
    >+                  Add 1 to Cell #3
    >-                  Subtract 1 from Cell #4
    >>+                 Add 1 to Cell #6
    [<]                 Move back to the first zero cell you find; this will
                        be Cell #1 which was cleared by the previous loop
    <-                  Decrement the loop Counter in Cell #0
]                       Loop until Cell #0 is zero; number of iterations is 8

The result of this is:
Cell no :   0   1   2   3   4   5   6
Contents:   0   0  72 104  88  32   8
Pointer :   ^

>>.                     Cell #2 has value 72 which is 'H'
>---.                   Subtract 3 from Cell #3 to get 101 which is 'e'
+++++++..+++.           Likewise for 'llo' from Cell #3
>>.                     Cell #5 is 32 for the space
<-.                     Subtract 1 from Cell #4 for 87 to give a 'W'
<.                      Cell #3 was set to 'o' from the end of 'Hello'
+++.------.--------.    Cell #3 for 'rl' and 'd'
>>+.                    Add 1 to Cell #5 gives us an exclamation point
>++.                    And finally a newline from Cell #6"#;

        let inter = Interpreter::build(code).unwrap();
        let mut out = MockInOut::dummy();
        inter.execute(&mut MockInOut::dummy(), &mut out).unwrap();

        assert_eq!(13, out.data.len());
        assert_eq!(
            "Hello World!\n".as_bytes(),
            out.data.iter().map(|c| *c).collect::<Vec<u8>>()
        );
    }

    #[test]
    fn test_data_overflow() {
        let code = "<";
        let inter = Interpreter::build(code).unwrap();
        let err = inter
            .execute(&mut MockInOut::dummy(), &mut MockInOut::dummy())
            .unwrap_err();
        assert_eq!("data overflow, idx = -1", err.to_string());

        let code = String::from_utf8(Vec::from([b'>'; 30000])).unwrap();
        let inter = Interpreter::build(&code).unwrap();
        let err = inter
            .execute(&mut MockInOut::dummy(), &mut MockInOut::dummy())
            .unwrap_err();
        assert_eq!("data overflow, idx = 30000", err.to_string());
    }
}
