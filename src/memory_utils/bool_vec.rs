/*
  ____                 __               __ __                 __
 / __ \__ _____ ____  / /___ ____ _    / //_/__ _______  ___ / /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / ,< / -_) __/ _ \/ -_) /
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /_/|_|\__/_/ /_//_/\__/_/
  Part of the Quantum OS Kernel

Copyright 2022 Gavin Kellam

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT
OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

*/

use crate::bitset::BitSet;
use crate::error_utils::QuantumError;

pub struct BoolVec<'a> {
    buffer: &'a mut [u8]
}

impl<'a> BoolVec<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer
        }
    }

    pub fn transfer_expand(&mut self, buffer: &'a mut [u8]) -> Result<(), QuantumError> {
        if buffer.len() >= self.buffer.len() {

            // transfer all bytes to new buffer
            for i in 0..self.buffer.len() {
                buffer[i] = self.buffer[i];
            }

            self.buffer = buffer;

            return Ok(());
        }

        Err(QuantumError::NoSpaceRemaining)
    }

    pub fn set_bit(&mut self, bit_id: usize, value: bool) -> Result<(), QuantumError> {
        if (bit_id / 8) > self.buffer.len() {
            self.buffer[bit_id / 8].set_bit((bit_id % 8) as u8, value);
        }

        Err(QuantumError::IndexOutOfRange)
    }

    pub fn len(&self) -> usize {
        self.buffer.len() * 8
    }
}