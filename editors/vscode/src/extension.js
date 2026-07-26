const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

function activate(context) {
    console.log('Calesteria extension activated');

    // Run compiler binary as LSP server
    const serverOptions = {
        command: 'calesteria',
        args: ['lsp']
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'calesteria' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.caria')
        }
    };

    client = new LanguageClient(
        'calesteriaLsp',
        'Calesteria Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

module.exports = {
    activate,
    deactivate
};
