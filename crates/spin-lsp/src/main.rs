use tower_lsp::{LspService, Server};

mod backend;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(backend::SpinBackend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
