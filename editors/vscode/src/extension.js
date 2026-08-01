const vscode = require('vscode');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');
const path = require('path');
const { CariaDocumentFormatter } = require('./formatter');

/** @type {LanguageClient} */
let client;

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    console.log('Caria language extension is now active');

    // Register built-in formatter
    context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider(
            [{ scheme: 'file', language: 'caria' }, { scheme: 'file', language: 'caria-html' }],
            new CariaDocumentFormatter()
        )
    );

    const config = vscode.workspace.getConfiguration('caria');
    const compilerPath = config.get('compilerPath') || 'caria';

    // The server is implemented in the Caria compiler binary
    const serverOptions = {
        run: {
            command: compilerPath,
            args: ['lsp'],
            transport: TransportKind.stdio
        },
        debug: {
            command: compilerPath,
            args: ['lsp', '--debug'],
            transport: TransportKind.stdio
        }
    };

    // Options to control the language client
    const clientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'caria' },
            { scheme: 'file', language: 'caria-html' }
        ],
        synchronize: {
            // Notify the server about file changes to '.caria' files contained in the workspace
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.caria')
        },
        initializationOptions: {
            enableFormatting: config.get('enableFormatting'),
            formatterTabSize: config.get('formatterTabSize'),
            enableTypeChecking: config.get('enableTypeChecking')
        }
    };

    // Create the language client and start the client.
    client = new LanguageClient(
        'cariaLsp',
        'Caria Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client. This will also launch the server
    client.start();

    // Register Commands
    context.subscriptions.push(
        vscode.commands.registerCommand('caria.compile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Caria file to compile');
                return;
            }

            const doc = editor.document;
            if (doc.languageId !== 'caria') {
                vscode.window.showErrorMessage('Active file is not a Caria file');
                return;
            }

            if (doc.isDirty) {
                await doc.save();
            }

            runTerminalCommand(`caria build "${doc.fileName}"`);
        }),

        vscode.commands.registerCommand('caria.run', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Caria file to run');
                return;
            }

            const doc = editor.document;
            if (doc.isDirty) {
                await doc.save();
            }

            runTerminalCommand(`caria run "${doc.fileName}"`);
        }),

        vscode.commands.registerCommand('caria.buildProject', () => {
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders) {
                vscode.window.showErrorMessage('No workspace open');
                return;
            }

            runTerminalCommand('caria build');
        }),

        vscode.commands.registerCommand('caria.test', async () => {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'caria') {
                if (editor.document.isDirty) {
                    await editor.document.save();
                }
                runTerminalCommand(`caria test "${editor.document.fileName}"`);
            } else {
                runTerminalCommand('caria test');
            }
        }),

        vscode.commands.registerCommand('caria.check', () => {
            runTerminalCommand('caria check');
        })
    );

    // Listen for configuration changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(e => {
            if (e.affectsConfiguration('caria')) {
                // If compiler path changed, we need to restart the server
                if (e.affectsConfiguration('caria.compilerPath')) {
                    vscode.window.showInformationMessage('Caria compiler path changed. Restarting Language Server...', 'Restart')
                        .then(selection => {
                            if (selection === 'Restart') {
                                vscode.commands.executeCommand('workbench.action.reloadWindow');
                            }
                        });
                } else if (client) {
                    // Send updated config to server
                    const newConfig = vscode.workspace.getConfiguration('caria');
                    client.sendNotification('workspace/didChangeConfiguration', {
                        settings: {
                            caria: {
                                enableFormatting: newConfig.get('enableFormatting'),
                                formatterTabSize: newConfig.get('formatterTabSize'),
                                enableTypeChecking: newConfig.get('enableTypeChecking')
                            }
                        }
                    });
                }
            }
        })
    );
}

/**
 * Runs a command in the Caria terminal
 * @param {string} command 
 */
function runTerminalCommand(command) {
    const terminalName = 'Caria';
    let terminal = vscode.window.terminals.find(t => t.name === terminalName);

    if (!terminal) {
        terminal = vscode.window.createTerminal(terminalName);
    }

    terminal.show();
    terminal.sendText(command);
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
