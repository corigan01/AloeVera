/*
  ____                 __               __   _ __
 / __ \__ _____ ____  / /___ ____ _    / /  (_) /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / /__/ / _ \
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /____/_/_.__/
  Part of the Quantum OS Project

Copyright 2023 Gavin Kellam

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

use crate::debug::SimpleStreamFunction;
use core::fmt::{Debug, Write};
use core::marker::PhantomData;

pub enum StreamType {
    BufferBacked,
    Console,
    Serial,
    Other,
}

//#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct StreamConnection {
    pub(crate) info: StreamConnectionInformation,
    pub(crate) outlet: Option<&'static mut (dyn Write + Send + Sync)>,
    pub(crate) ignore_welcome: bool,
    pub(crate) simple_outlet: Option<SimpleStreamFunction>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StreamConnectionInformation {
    pub(crate) max_chars: Option<(usize, usize)>,
    pub(crate) does_support_scrolling: bool,
    pub(crate) data_rate: Option<usize>,
    pub(crate) connection_name: &'static str,
    pub(crate) who_using: &'static str,
}

impl StreamConnectionInformation {
    pub fn new() -> Self {
        Self {
            max_chars: None,
            does_support_scrolling: true,
            data_rate: None,
            connection_name: "Unknown",
            who_using: "Unknown",
        }
    }
}

impl Default for StreamConnection {
    fn default() -> Self {
        Self {
            info: StreamConnectionInformation::new(),
            outlet: None,
            ignore_welcome: false,
            simple_outlet: None,
        }
    }
}

pub struct UnknownConnectionType;
pub struct ConsoleStreamType;

pub struct StreamConnectionBuilder<Type = UnknownConnectionType> {
    info: StreamConnectionInformation,
    outlet: Option<&'static mut (dyn Write + Send + Sync)>,
    simple_outlet: Option<SimpleStreamFunction>,
    ignore_welcome: bool,
    reserved: PhantomData<Type>,
}

impl StreamConnectionBuilder {
    pub fn new() -> StreamConnectionBuilder<UnknownConnectionType> {
        StreamConnectionBuilder {
            info: StreamConnectionInformation::new(),
            outlet: None,
            simple_outlet: None,
            ignore_welcome: false,
            reserved: Default::default(),
        }
    }
}

impl StreamConnectionBuilder<UnknownConnectionType> {
    pub fn console_connection(self) -> StreamConnectionBuilder<ConsoleStreamType> {
        StreamConnectionBuilder {
            info: self.info,
            outlet: self.outlet,
            simple_outlet: None,
            ignore_welcome: false,
            reserved: Default::default(),
        }
    }
}

impl StreamConnectionBuilder<ConsoleStreamType> {
    pub fn add_simple_outlet(mut self, outlet: SimpleStreamFunction) -> Self {
        self.simple_outlet = Some(outlet);

        self
    }

    pub fn add_outlet(mut self, outlet: &'static mut (dyn Write + Send + Sync)) -> Self {
        self.outlet = Some(outlet);

        self
    }

    pub fn add_max_chars(mut self, max_x: usize, max_y: usize) -> Self {
        self.info.max_chars = Some((max_x, max_y));

        self
    }

    pub fn add_connection_name(mut self, name: &'static str) -> Self {
        self.info.connection_name = name;

        self
    }

    pub fn add_who_using(mut self, name: &'static str) -> Self {
        self.info.who_using = name;

        self
    }

    pub fn add_max_data_rate(mut self, data_rate: usize) -> Self {
        self.info.data_rate = Some(data_rate);

        self
    }

    pub fn does_support_scrolling(mut self, scrolling: bool) -> Self {
        self.info.does_support_scrolling = scrolling;

        self
    }

    pub fn prevent_welcome_message(mut self, flag: bool) -> Self {
        self.ignore_welcome = flag;

        self
    }

    pub fn build(self) -> StreamConnection {
        StreamConnection {
            info: self.info,
            outlet: self.outlet,
            ignore_welcome: self.ignore_welcome,
            simple_outlet: self.simple_outlet,
        }
    }
}
