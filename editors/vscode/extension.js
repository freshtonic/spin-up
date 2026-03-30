const { workspace } = require("vscode");
const {
  LanguageClient,
  TransportKind,
} = require("vscode-languageclient/node");

let client;

function activate(context) {
  const config = workspace.getConfiguration("spin.lsp");
  const command = config.get("path", "spin-lsp");

  const serverOptions = {
    command,
    transport: TransportKind.stdio,
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "spin" }],
  };

  client = new LanguageClient(
    "spin-lsp",
    "Spin Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
}

function deactivate() {
  if (client) {
    return client.stop();
  }
}

module.exports = { activate, deactivate };
