# Publishing the extension

The short way: pack it with `./install.sh`, then upload `gesu-lang.vsix` at
https://marketplace.visualstudio.com/manage (the `...` menu of the extension, Update).
Raise `version` in `package.json` first: the Marketplace refuses a version it has seen.

From the terminal:

The `publisher` in `package.json` must be the name of your publisher.

VS Code Marketplace:

1. Create a publisher at https://marketplace.visualstudio.com/manage.
2. Create a Personal Access Token in Azure DevOps with the scope "Marketplace: Manage".
3. `npx @vscode/vsce login <publisher>` and paste the token.
4. `npx @vscode/vsce publish`.

Open VSX, which Cursor and VSCodium read:

1. Create an account and a token at https://open-vsx.org, and a namespace with the publisher's name.
2. `npx ovsx publish gesu-lang.vsix -p <token>`.

The steps change now and then. The current ones are at
https://code.visualstudio.com/api/working-with-extensions/publishing-extension.
