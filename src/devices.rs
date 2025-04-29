#[derive(Debug)]
pub enum Error {
    Write(String),
    Read(String),
}

pub trait Device {
    fn write(&mut self, value: u16) -> Result<(), Error>;
    fn read(&mut self) -> Result<Option<u16>, Error>;
}

pub struct Buffer(pub Vec<u16>);

impl Device for Buffer {
    fn write(&mut self, value: u16) -> Result<(), Error> {
        self.0.push(value);
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u16>, Error> {
        Ok(self.0.pop())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_trait() {
        let mut b = Buffer(vec![]);
        b.write(5).unwrap();
        assert_eq!(b.read().unwrap(), Some(5));
    }
}
