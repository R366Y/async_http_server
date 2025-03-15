use std::error::Error;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

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
            // Add a 30 seconds timeout for handling each connection
            match timeout(Duration::from_secs(30), handle_connection(socket)).await {
                Ok(result) => {
                    // Process the connection
                    if let Err(e) = result {
                        eprintln!("Error handling connection: {}", e);
                    }
                },
                Err(_) => {
                    eprintln!("Connection handling time out");
                }
            }
            

        });
    }
}

async fn handle_connection(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    // Create a buffer to store the request
    let mut buffer = vec![0u8; 8192]; // 8KB buffer
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut request = httparse::Request::new(&mut headers);

    // Read bytes from the socket
    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    // Parse the request
    match request.parse(&buffer[..n]) {
        Ok(httparse::Status::Complete(_size)) => {
            // Successfully parsed the request
            let method = request.method.unwrap_or("");
            let path = request.path.unwrap_or("");

            println!("Received {} request for {}", method, path);

            match method {
                "GET" => handle_get_request(socket, path).await?,
                _ => {
                    // Respond with 405 Method Not Allowed
                    let response = "HTTP/1.1 405 Method Not Allowed\r\n\r\n";
                    socket.write_all(response.as_bytes()).await?;
                }
            }
        }
        Ok(httparse::Status::Partial) => {
            // Incomplete request
            let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 26\r\n\r\nIncomplete request received";
            socket.write_all(response.as_bytes()).await?;
        }
        Err(_) => {
            // Malformed request
            let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 24\r\n\r\nMalformed HTTP request";
            socket.write_all(response.as_bytes()).await?;
        }
    }
    Ok(())
}

async fn handle_get_request(
    mut socket: TcpStream,
    path: &str,
) -> Result<(), Box<dyn Error>> {

    // Create a simple router from different paths 
    let (status_line, content_type, content) = match path {
        "/" => (
            "HTTP/1.1 200 OK",
            "text/html",
            "<html><body>
                <h1>Welcome to Tokio Async Server</h1>
                <p>This is a simple async HTTP server built with Tokio.</p>
                <ul>
                    <li><a href='/'>Home</a></li>
                    <li><a href='/about'>About</a></li>
                    <li><a href='/async'>Async Info</a></li>
                </ul>
            </body></html>"
        ),
        "/about" => (
            "HTTP/1.1 200 OK",
            "text/html",
            "<html><body>
                <h1>About This Server</h1>
                <p>This is a demonstration of asynchronous programming in Rust using Tokio.</p>
                <p><a href='/'>Back to home</a></p>
            </body></html>"
        ),
        "/async" => (
            "HTTP/1.1 200 OK",
            "text/html",
            "<html><body>
                <h1>Async Programming</h1>
                <p>This server uses Tokio's async runtime to handle multiple connections efficiently.</p>
                <p>Key concepts:</p>
                <ul>
                    <li>Non-blocking I/O</li>
                    <li>Task-based concurrency</li>
                    <li>Event-driven architecture</li>
                </ul>
                <p><a href='/'>Back to home</a></p>
            </body></html>"
        ),
        _ => (
            "HTTP/1.1 404 NOT FOUND",
            "text/html",
            "<html><body>
                <h1>404: Page not found</h1>
                <p>The requested resource could not be found.</p>
                <p><a href='/'>Back to home</a></p>
            </body></html>"
        ),
    };

    // Construct the full response
    let response = format!(
        "{}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n{}",
        status_line,
        content.len(),
        content_type,
        content
    );
    // Write the response asynchronously
    socket.write_all(response.as_bytes()).await?;

    Ok(())
}
