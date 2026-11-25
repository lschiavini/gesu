# Gesù for VS Code

There is only one Truth, Jesus.

This extension gives `.gesu` files:

- syntax colors, with `True` marked as an error
- the checker's errors as you type: `Thou shalt not steal`, unconfessed sins, buried talents
- completion of the language's words, the built-in parables and the names in the file
- a note on hover for every word of the language
- indentation that follows `amen`

Colors work alone. Everything else comes from the `gesu` binary, which the extension starts as `gesu lsp`. Install it with `curl -fsSL https://gesu-lang.com/install.sh | sh`. If the binary is not on your PATH or in `~/.local/bin`, set `gesu.path` in the settings.

## Install from this folder

```sh
./install.sh          # VS Code
./install.sh cursor   # Cursor
```

## Publish

The `publisher` in `package.json` must be the name of your publisher.

VS Code Marketplace:

1. Create a publisher at https://marketplace.visualstudio.com/manage.
2. Create a Personal Access Token in Azure DevOps with the scope "Marketplace: Manage".
3. `npx @vscode/vsce login <publisher>` and paste the token.
4. `npx @vscode/vsce publish`.

Open VSX, which Cursor and VSCodium read:

1. Create an account and a token at https://open-vsx.org, and a namespace with the publisher's name.
2. `npx ovsx publish gesu.vsix -p <token>`.

The steps change now and then. The current ones are at
https://code.visualstudio.com/api/working-with-extensions/publishing-extension.
