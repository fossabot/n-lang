use std::{
    cell::{
        RefCell,
        RefMut,
    },
    fmt::{
        self,
        Display,
        Write,
    },
    rc::Rc,
};

#[derive(Debug)]
pub struct CodeFormatter<'a, T: 'a + Write> {
    target: &'a mut T,
    pub indent_size: usize,
}

impl<'a, T: 'a + Write> CodeFormatter<'a, T> {
    #[inline]
    pub fn new(target: &'a mut T) -> Self {
        Self {
            target,
            indent_size: 1,
        }
    }
    #[inline]
    fn write_indent(&mut self, indent_level: usize) -> fmt::Result {
        for _ in 0..indent_level {
            for _ in 0..self.indent_size {
                self.target.write_char(' ')?;
            }
        }
        Ok(())
    }
    #[inline]
    fn write(&mut self, value: impl Display) -> fmt::Result {
        write!(&mut self.target, "{}", value)
    }
    #[inline]
    fn end_line(&mut self) -> fmt::Result {
        writeln!(&mut self.target, "")
    }
    #[inline]
    pub fn write_line(&mut self, indent_level: usize, value: impl Display) -> fmt::Result {
        self.write_indent(indent_level)?;
        self.write(value)?;
        self.end_line()
    }
    #[inline]
    pub fn root_block(self) -> BlockFormatter<'a, T> {
        let block = BlockFormatter {
            target: Rc::new(RefCell::new(self)),
            indent_level: 0,
        };
        block
    }
    #[inline]
    pub fn target(&mut self) -> &mut T {
        &mut self.target
    }
}

#[derive(Debug)]
pub struct BlockFormatter<'a, T: 'a + Write> {
    target: Rc<RefCell<CodeFormatter<'a, T>>>,
    indent_level: usize,
}

impl<'a, T: 'a + Write> BlockFormatter<'a, T> {
    #[inline]
    pub fn formatter(&mut self) -> RefMut<CodeFormatter<'a, T>> {
        self.target.borrow_mut()
    }
    #[inline]
    pub unsafe fn write(&mut self, v: impl Display) -> fmt::Result {
        let mut f = self.target.borrow_mut();
        f.write(v)
    }
    #[inline]
    pub fn write_line(&mut self, line: impl Display) -> fmt::Result {
        let mut f = self.target.borrow_mut();
        f.write_line(self.indent_level, line)
    }
    #[inline]
    pub fn sub_block(&self) -> Self {
        Self {
            target: self.target.clone(),
            indent_level: self.indent_level + 1,
        }
    }
    #[inline]
    pub fn line<'b>(&'b mut self) -> Result<LineFormatter<'a, 'b, T>, fmt::Error> {
        LineFormatter::new(self.indent_level, self.target.borrow_mut())
    }
}

impl<'a, T: 'a + Write> Clone for BlockFormatter<'a, T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            indent_level: self.indent_level,
        }
    }
}

#[derive(Debug)]
pub struct LineFormatter<'a, 'b, T: 'a + Write>
    where CodeFormatter<'a, T>: 'b {
    target: RefMut<'b, CodeFormatter<'a, T>>,
}

impl<'a, 'b, T: 'a + Write> LineFormatter<'a, 'b, T>
    where CodeFormatter<'a, T>: 'b
{
    #[inline]
    pub fn new(indent_level: usize, mut target: RefMut<'b, CodeFormatter<'a, T>>) -> Result<Self, fmt::Error> {
        target.write_indent(indent_level)?;
        Ok(Self {
            target,
        })
    }
    #[inline]
    pub fn write(&mut self, value: impl Display) -> fmt::Result {
        self.target.write(value)
    }
}

impl<'a, 'b, T: 'a + Write> Write for LineFormatter<'a, 'b, T>
    where CodeFormatter<'a, T>: 'b {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s)
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.write(c)
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        self.write(args)
    }
}

impl<'a, 'b, T: 'a + Write> Drop for LineFormatter<'a, 'b, T> {
    fn drop(&mut self) {
        let _ = self.target.end_line();
    }
}

pub struct SimpleFormatter<'a> {
    target: &'a mut dyn Write,
    buffer: String,
    top: bool,
    started: bool,
    indent_size: usize,
}

impl<'a> SimpleFormatter<'a> {
    #[inline]
    pub fn new(target: &'a mut dyn Write, indent_size: usize) -> Self {
        Self {
            target,
            buffer: String::new(),
            top: true,
            started: false,
            indent_size,
        }
    }
    #[inline]
    pub fn sub_block(&mut self) -> SimpleFormatter {
        let started = self.started;
        let indent_size = self.indent_size;
        SimpleFormatter {
            target: self,
            buffer: String::new(),
            top: false,
            started,
            indent_size,
        }
    }
    pub fn write(&mut self, value: impl fmt::Display) -> fmt::Result {
        if self.top {
            return self.target.write_fmt(format_args!("{}", value));
        }
        self.buffer.write_fmt(format_args!("{}", value))?;
        {
            let mut chunks = self.buffer.split('\n')
                .peekable();
            while let Some(chunk) = chunks.next() {
                if !chunk.is_empty() && !self.started {
                    for _ in 0..self.indent_size {
                        self.target.write_char(' ')?;
                    }
                    self.started = true;
                }
                self.target.write_str(chunk)?;
                if chunks.peek().is_some() {
                    self.target.write_char('\n')?;
                    self.started = false;
                }
            }
        }
        self.buffer.clear();
        Ok(())
    }
}

impl<'a> Write for SimpleFormatter<'a> {
    #[inline]
    fn write_str(&mut self, str: &str) -> fmt::Result {
        self.write(str)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.write(c)
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        self.write(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_class() -> fmt::Result {
        let mut result = String::new();
        {
            let mut block = {
                let mut f = CodeFormatter::new(&mut result);
                f.indent_size = 2;
                f.root_block()
            };
            block.write_line("class X {")?;
            let mut sub_block = block.sub_block();
            sub_block.write_line("function a () {")?;
            let mut sub_sub_block = sub_block.sub_block();
            sub_sub_block.write_line(format_args!("return {};", true))?;
            sub_block.write_line("}")?;
            block.write_line("}")?;
        }
        assert_eq!("class X {\
        \n  function a () {\
        \n    return true;\
        \n  }\
        \n}\
        \n", result);
        Ok(())
    }

    #[test]
    fn test_simple_f() -> fmt::Result {
        let mut result = String::new();
        {
            let mut f = SimpleFormatter::new(&mut result, 4);
            writeln!(f, "export interface Bla {{")?;
            {
                let mut sub_f = f.sub_block();
                writeln!(sub_f, "Bla, bla, bla.")?;
            }
            writeln!(f, "}}")?;
        }
        assert_eq!(result, "\
            export interface Bla {\
            \n    Bla, bla, bla.\
            \n}\
        \n");
        Ok(())
    }
}
