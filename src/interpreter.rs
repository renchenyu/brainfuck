use std::io;
use std::io::{Read, Write};

fn parse_inst(code: &str) -> Vec<u8> {
    code.as_bytes()
        .iter()
        .filter(|c| {
            let c = **c;
            c == b'>'
                || c == b'<'
                || c == b'+'
                || c == b'-'
                || c == b'.'
                || c == b','
                || c == b'['
                || c == b']'
        })
        .map(|c| *c)
        .collect::<Vec<u8>>()
}

pub struct Interpreter {
    inst: Vec<u8>,
    data: [u8; 30000],
    d_offset: usize,
    i_offset: usize,
}

impl Interpreter {
    pub fn new(code: &str) -> Self {
        Self {
            inst: parse_inst(code),
            data: [0; 30000],
            d_offset: 0,
            i_offset: 0,
        }
    }

    pub fn execute(&mut self) -> Vec<u8> {
        let mut result = vec![];
        while self.i_offset < self.inst.len() {
            let ins = self.inst[self.i_offset];
            match ins {
                b'>' => self.d_offset += 1,
                b'<' => self.d_offset -= 1,
                b'+' => self.incr(),
                b'-' => self.decr(),
                b'.' => result.push(self.data[self.d_offset]),
                b',' => {
                    print!("> ");
                    io::stdout().flush().unwrap();
                    let mut buf = [0; 2];
                    io::stdin().read_exact(&mut buf).unwrap();
                    self.data[self.d_offset] = buf[0];
                }
                b'[' => {
                    if self.data[self.d_offset] == 0 {
                        self.loop_end()
                    }
                }
                b']' => {
                    if self.data[self.d_offset] != 0 {
                        self.loop_back()
                    }
                }
                _ => panic!("unexpected instruction: {}", ins),
            }
            self.i_offset += 1;
        }
        result
    }

    fn loop_end(&mut self) {
        //cur is [
        let mut cnt = 1;
        for i in self.i_offset + 1..self.inst.len() {
            match self.inst[i] {
                b'[' => cnt += 1,
                b']' => {
                    cnt -= 1;
                    if cnt == 0 {
                        self.i_offset = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(self.inst[self.i_offset], b']', "bad code!");
    }

    fn loop_back(&mut self) {
        //cur is ]
        let mut cnt = 1;
        for i in (0..self.i_offset).rev() {
            match self.inst[i] {
                b']' => cnt += 1,
                b'[' => {
                    cnt -= 1;
                    if cnt == 0 {
                        self.i_offset = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(self.inst[self.i_offset], b'[', "bad code!");
    }

    fn incr(&mut self) {
        if self.data[self.d_offset] == u8::MAX {
            self.data[self.d_offset] = 0;
        } else {
            self.data[self.d_offset] += 1;
        }
    }

    fn decr(&mut self) {
        if self.data[self.d_offset] == 0 {
            self.data[self.d_offset] = u8::MAX;
        } else {
            self.data[self.d_offset] -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_end() {
        let mut i = Interpreter {
            inst: vec![b'[', b'[', b'>', b']', b']'],
            data: [0; 30000],
            d_offset: 0,
            i_offset: 0,
        };
        i.loop_end();
        assert_eq!(4, i.i_offset);
    }

    #[test]
    fn test_loop_back() {
        let mut i = Interpreter {
            inst: vec![b'[', b'[', b'>', b']', b']'],
            data: [0; 30000],
            d_offset: 0,
            i_offset: 4,
        };
        i.loop_back();
        assert_eq!(0, i.i_offset);
    }

    #[test]
    fn test_1() {
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
        let mut inter = Interpreter::new(code);
        let result = inter.execute();

        assert_eq!(1, result.len());
        assert_eq!(55, result[0]);
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

        let mut inter = Interpreter::new(code);
        let result = inter.execute();

        assert_eq!(13, result.len());
        assert_eq!("Hello World!\n".as_bytes(), result);
    }
}
