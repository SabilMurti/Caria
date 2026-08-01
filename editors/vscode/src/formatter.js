const vscode = require('vscode');

class CariaDocumentFormatter {
    provideDocumentFormattingEdits(document, options, token) {
        const edits = [];
        let indentLevel = 0;
        const indentString = options.insertSpaces ? ' '.repeat(options.tabSize) : '\t';
        
        for (let i = 0; i < document.lineCount; i++) {
            const line = document.lineAt(i);
            const text = line.text.trim();
            
            // Decrease indent for closing brackets on this line
            if (text.match(/^[\}\]\)]/)) {
                indentLevel = Math.max(0, indentLevel - 1);
            }
            
            const currentIndent = indentString.repeat(indentLevel);
            
            // If the line has incorrect indentation, replace the leading whitespace
            const leadingWhitespace = line.text.match(/^\s*/)[0];
            if (leadingWhitespace !== currentIndent && text.length > 0) {
                edits.push(vscode.TextEdit.replace(
                    new vscode.Range(i, 0, i, leadingWhitespace.length),
                    currentIndent
                ));
            }
            
            // Increase indent for next lines if this line opens a block
            const openBrackets = (text.match(/[\{\[\(]/g) || []).length;
            const closeBrackets = (text.match(/[\}\]\)]/g) || []).length;
            indentLevel += (openBrackets - closeBrackets);
            
            // Prevent indentLevel from going below 0 in malformed code
            indentLevel = Math.max(0, indentLevel);
        }
        
        return edits;
    }
}

module.exports = {
    CariaDocumentFormatter
};
