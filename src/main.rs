use std::error::Error;
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
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
                }
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
            let response =
                "HTTP/1.1 400 Bad Request\r\nContent-Length: 26\r\n\r\nIncomplete request received";
            socket.write_all(response.as_bytes()).await?;
        }
        Err(_) => {
            // Malformed request
            let response =
                "HTTP/1.1 400 Bad Request\r\nContent-Length: 24\r\n\r\nMalformed HTTP request";
            socket.write_all(response.as_bytes()).await?;
        }
    }
    Ok(())
}

async fn handle_get_request(mut socket: TcpStream, path: &str) -> Result<(), Box<dyn Error>> {
    // Create a simple router from different paths
     return match path {
         "/" => {
             serve_static_html(
                 & mut socket,
                 "<html><body>
                    <h1>Welcome to Tokio Async Server</h1>
                    <p>This is a simple async HTTP server built with Tokio.</p>
                    <ul>
                        <li><a href='/'>Home</a></li>
                        <li><a href='/about'>About</a></li>
                        <li><a href='/files/index.html'>Static File Example</a></li>
                        <li><a href='/files/'>Files Directory</a></li>
                    </ul>
                </body></html>",
                 "HTTP/1.1 200 OK"
             ).await
         },
         "/about" => {
             serve_static_html(
                 &mut socket,
                 "<html><body>
                    <h1>About This Server</h1>
                    <p>This is a demonstration of asynchronous programming in Rust using Tokio.</p>
                    <p><a href='/'>Back to home</a></p>
                </body></html>"
                 , "HTTP/1.1 200 OK",
             ).await
         },
         _ if path.starts_with("/files/") => {
             // Handle file requests
             serve_file(&mut socket, path).await
         },
         _ => {
             serve_static_html(
                 &mut socket,
                 "<html><body>
                    <h1>404: Page not found</h1>
                    <p>The requested resource could not be found.</p>
                    <p><a href='/'>Back to home</a></p>
                </body></html>",
                 "HTTP/1.1 404 NOT FOUND"
             ).await
         },
     };
}

// Helper function to serve static HTML content
async fn serve_static_html(socket: &mut TcpStream, content: &str, status: &str) -> Result<(), Box<dyn Error>> {
    let status_line = status.to_string();
    let content_type = "text/html".to_string();

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

// Helper function to serve files
async fn serve_file(socket: & mut TcpStream, path: &str) -> Result<(), Box<dyn Error>> {
    // Extract the file path from the URL
    let file_path = path.trim_start_matches("/files/");

    // For security, ensure the path doesn't contain '..'
    // to prevent directory traversal
    if file_path.contains("..") {
        serve_static_html(
            socket,
            "<html><body><h1>403 Forbidden</h1><p>Access denied.</p></body></html>",
            "HTTP/1.1 403 Forbidden"
        ).await?
    }

    // Construct the full path (relative to a 'public' directory)
    let file_path = Path::new("public").join(file_path);

    // Try to open file asynchronously
    match File::open(&file_path).await {
        Ok(mut file) => {
            // Read the file content
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).await?;

            // Determine content type based on file extension
            let content_type = match file_path.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html",
                Some("css") => "text/css",
                Some("js") => "application/javascript",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("png") => "image/png",
                Some("gif") => "image/gif",
                _ => "application/octet-stream",
            };

            // Construct and send the response
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
                contents.len(),
                content_type
            );

            socket.write_all(response.as_bytes()).await?;
            socket.write_all(&contents).await?;
        },
        Err(_) => {
            // File not found
            serve_static_html(
                socket,
                "<html><body><h1>404 Not Found</h1><p>The requested file could not be found.</p></body></html>",
                "HTTP/1.1 404 NOT FOUND"
            ).await?
        }
    }

    Ok(())
}