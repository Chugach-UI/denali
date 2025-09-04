use denali_server_core::socket::Socket;

pub struct DisplaySocket {
    socket: Socket,
    display: WlDisplay,
}

// TEMP until codegen is done
struct WlDisplay;
