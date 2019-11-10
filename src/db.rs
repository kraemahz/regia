use std::fs::File;
use std::io::{
    BufReader, BufWriter, Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult,
    Write,
};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::note::Notes;
use crate::todo::Tasks;

pub fn write_to_disk<P: AsRef<Path>>(path: P, buf: &[u8]) -> Result<(), IOError> {
    let file = File::create(path)?;
    let mut stream = BufWriter::new(file);
    stream.write_all(&buf)
}

pub fn read_from_disk<P: AsRef<Path>>(path: P) -> IOResult<Vec<u8>> {
    let file = File::open(path)?;
    let mut stream = BufReader::new(file);
    let mut data = Vec::new();
    stream.read_to_end(&mut data)?;
    Ok(data)
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Database {
    pub(crate) tasks: Tasks,
    pub(crate) notes: Notes,
}

impl Database {
    pub fn serialize_msgpack(&self) -> Result<Vec<u8>, IOError> {
        let mut buf = Vec::new();
        match self.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
            Ok(_) => Ok(buf),
            Err(_) => Err(IOError::new(IOErrorKind::Other, "Serialization failed")),
        }
    }

    pub fn deserialize_msgpack(buf: &[u8]) -> Result<Database, IOError> {
        let mut de = rmp_serde::Deserializer::new(&buf[..]);
        match Database::deserialize(&mut de) {
            Ok(tasks) => Ok(tasks),
            Err(_) => Err(IOError::new(IOErrorKind::Other, "Deserialization failed")),
        }
    }

    pub fn from_disk<P: AsRef<Path>>(path: P) -> Result<Database, IOError> {
        let buf = read_from_disk(path)?;
        Database::deserialize_msgpack(buf.as_slice())
    }

    pub fn to_disk<P: AsRef<Path>>(&self, path: P) -> Result<(), IOError> {
        let buf = self.serialize_msgpack()?;
        write_to_disk(path, buf.as_slice())
    }
}
