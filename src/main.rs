use log::debug;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Bind to a port
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server listening on port 8080");

    loop {
        // The .await make this non-blocking
        let (socket, addr) = listener.accept().await?;
        println!("Accepted connection from: {}", addr);

        // Spawn a new task for each connection
        tokio::spawn(async move {
            // Process the connection
            if let Err(e) = handle_connection(socket).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    // Create a buffer to store the request
    let mut buffer = [0; 1204];

    // Read bytes from the socket
    let n = socket.read(&mut buffer).await?;

    // Convert the request bytes to a string
    let request = String::from_utf8_lossy(&buffer[..n]);

    //Print the incoming request
    debug!("Received request: {}", request);

    // Check if it's a GET request
    let first_line = request.lines().next().unwrap_or("");
    if first_line.starts_with("GET") {
        handle_get_request(socket, first_line).await?;
    } else {
        // Respond with 405 Method Not Allowed
        let response = "HTTP/1.1 405 Method Not Allowed\r\n\r\n";
        socket.write_all(response.as_bytes()).await?;
    }
    Ok(())
}

async fn handle_get_request(
    mut socket: TcpStream,
    request_line: &str,
) -> Result<(), Box<dyn Error>> {
    // Extract the path from the request line
    // Format: GET /path HTTP/1.1
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let path = parts.get(1).unwrap_or(&"/");

    let (status_line, content) = match *path {
        "/" => (
            "HTTP/1.1 200 OK",
            "<html><body><h1>Welcome to Tokio Async Server</h1></body></html>",
        ),
        "/about" => (
            "HTTP/1.1 200 OK",
            "<html><body><h1>About Page</h1></body></html>",
        ),
        _ => (
            "HTTP/1.1 404 NOT FOUND",
            "<html><body><h1>404: Page not found</h1></body></html>",
        ),
    };

    // Construct the full response
    let response = format!(
        "{}\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}",
        status_line,
        content.len(),
        content
    );

    // Write the response asynchronously
    socket.write_all(response.as_bytes()).await?;

    Ok(())
}
