use crate::socket::Socket;

pub struct DisplaySocket {
    socket: Socket,
    display: WlDisplay,
}

// TEMP until codegen is done
struct WlDisplay;
