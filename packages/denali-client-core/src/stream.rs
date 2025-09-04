use std::{os::fd::OwnedFd, path::Path};

pub struct Stream {
    fd: OwnedFd,
}

impl Stream {
    pub fn connect(path: &Path) -> Self {
        todo!()
    }

    pub fn from_fd(fd: OwnedFd) -> Self {
        Self { fd }
    }

    pub fn into_split(self) -> (ReadStream, WriteStream) {
        let fd = self.fd;
        let dup = fd.try_clone().unwrap();

        (ReadStream::from_fd(fd), WriteStream::from_fd(dup))
    }
}

pub struct ReadStream {
    fd: OwnedFd,
}

impl ReadStream {
    pub fn from_fd(fd: OwnedFd) -> Self {
        Self { fd }
    }

    pub async fn is_readable(&self) -> bool {
        todo!()
    }

    pub async fn read(&self, buf: &mut [u8]) -> usize {
        _ = buf;
        todo!()
    }

    pub fn try_read(&self, buf: &mut [u8]) -> usize {
        _ = buf;
        todo!()
    }

    pub async fn read_with_ancillary(
        &self,
        buf: &mut [u8],
        ancillary_buf: &mut [i32],
    ) -> (usize, usize) {
        _ = buf;
        _ = ancillary_buf;
        todo!()
    }

    pub fn try_read_with_ancillary(
        &self,
        buf: &mut [u8],
        ancillary_buf: &mut [i32],
    ) -> (usize, usize) {
        _ = buf;
        _ = ancillary_buf;
        todo!()
    }
}

pub struct WriteStream {
    fd: OwnedFd,
}

impl WriteStream {
    pub fn from_fd(fd: OwnedFd) -> Self {
        Self { fd }
    }

    pub async fn is_writable(&self) -> bool {
        todo!()
    }

    pub async fn write(&self, buf: &[u8]) -> usize {
        _ = buf;
        todo!()
    }

    pub fn try_write(&self, buf: &[u8]) -> usize {
        _ = buf;
        todo!()
    }

    pub async fn write_all(&self, buf: &[u8]) {
        _ = buf;
        todo!()
    }

    pub fn try_write_all(&self, buf: &[u8]) {
        _ = buf;
        todo!()
    }

    pub async fn write_with_ancillary(&self, buf: &[u8], ancillary_buf: &[i32]) -> (usize, usize) {
        _ = buf;
        _ = ancillary_buf;
        todo!()
    }

    pub fn try_write_with_ancillary(
        &self,
        buf: &mut [u8],
        ancillary_buf: &mut [i32],
    ) -> (usize, usize) {
        _ = buf;
        _ = ancillary_buf;
        todo!()
    }

    pub async fn write_all_with_ancillary(&self, buf: &[u8], ancillary_buf: &[i32]) {
        _ = buf;
        _ = ancillary_buf;
        todo!()
    }

    pub fn try_write_all_with_ancillary(&self, buf: &mut [u8], ancillary_buf: &mut [i32]) {
        _ = buf;
        _ = ancillary_buf;
        todo!()
    }
}
