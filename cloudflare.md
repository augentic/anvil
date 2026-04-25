# Publishing Specify Docs via Cloudflare Pages + Access

Private mdbook documentation for the Specify Operator Guide, deployed to
Cloudflare Pages and gated behind Cloudflare Access with GitHub OAuth.

> **Why not GitHub Pages?** Private GitHub Pages visibility requires
> Enterprise Cloud. On Teams plans, Pages sites are publicly accessible
> regardless of repo visibility.

## Architecture

- GitHub Actions builds the mdbook on push to `main`
- Wrangler CLI deploys the built output to Cloudflare Pages
- A Cloudflare Access policy gates the site behind GitHub OAuth, restricted
  to members of the `augentic` org
- Cloudflare Access free tier covers up to 50 users

## GitHub Actions workflow

File: `.github/workflows/docs.yaml`

```yaml
name: Docs

on:
  push:
    branches: [main]
    paths:
      - "docs/**"
      - ".github/workflows/docs.yaml"
  workflow_dispatch:

concurrency:
  group: docs-${{ github.ref }}
  cancel-in-progress: true

jobs:
  deploy:
    name: Build and deploy documentation
    runs-on: ubuntu-latest
    timeout-minutes: 10
    permissions:
      contents: read
    steps:
      - name: Checkout repository
        uses: actions/checkout@v6

      - name: Install mdbook and preprocessors
        run: |
          mkdir -p "$HOME/.local/bin"
          curl -sSL https://github.com/rust-lang/mdBook/releases/latest/download/mdbook-x86_64-unknown-linux-gnu.tar.gz \
            | tar xz -C "$HOME/.local/bin"
          curl -sSL https://github.com/badboy/mdbook-mermaid/releases/latest/download/mdbook-mermaid-x86_64-unknown-linux-gnu.tar.gz \
            | tar xz -C "$HOME/.local/bin"
          echo "$HOME/.local/bin" >> "$GITHUB_PATH"

      - name: Build book
        run: mdbook build docs

      - name: Deploy to Cloudflare Pages
        uses: cloudflare/wrangler-action@v3
        with:
          apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          accountId: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
          command: pages deploy docs/book --project-name=specify-docs
```

## Cloudflare one-time setup

### 1. Create the Pages project

In the Cloudflare dashboard, go to **Workers & Pages** > **Create** > **Pages**.
Name it `specify-docs`. You can skip the initial deploy — the first GitHub
Actions run will create the deployment automatically via wrangler.

### 2. Add GitHub as an identity provider

1. Go to [GitHub](https://github.com/) > **Settings** > **Developer Settings** >
   **OAuth Apps** > **New OAuth app**.
2. Set:
   - **Application name**: something your team will recognise on the login page
     (e.g. "Augentic Docs")
   - **Homepage URL**: `https://<your-team-name>.cloudflareaccess.com`
   - **Authorization callback URL**:
     `https://<your-team-name>.cloudflareaccess.com/cdn-cgi/access/callback`
3. Register the app, note the **Client ID**, then generate and copy a
   **Client Secret**.
4. In [Cloudflare One](https://one.dash.cloudflare.com/), go to
   **Integrations** > **Identity providers**.
5. Select **Add new identity provider** > **GitHub**.
6. Paste the Client ID and Client Secret, then **Save**.
7. Select **Finish setup** — you will be asked to authorize Cloudflare Access
   for org/team read and email read permissions.

You can find your team name under **Settings** > **Team name and domain** >
**Team name** in the Zero Trust dashboard.

### 3. Create a reusable Access policy

1. In Cloudflare One, go to **Zero Trust** > **Access controls** > **Policies**.
2. Select **Add a policy**.
3. Enter a name (e.g. "Augentic org members").
4. Set Action to **Allow**.
5. Under Rules, add an Include rule: **Login Methods** > **GitHub** with
   **GitHub Organizations** = `augentic`.
6. **Save**.

### 4. Add the Pages site as an Access application

1. In Cloudflare One, go to **Zero Trust** > **Access controls** > **Applications**.
2. Select **Add an application** > **Self-hosted**.
3. Enter a name (e.g. "Specify Docs").
4. Under **Add public hostname**, select `specify-docs.pages.dev` from the
   Domain dropdown.
5. Attach the reusable policy you created in step 3.
6. Under Identity providers, enable **GitHub** and optionally turn on
   **Instant Auth** (skips the Cloudflare login page and redirects straight
   to GitHub OAuth).
7. **Save**.

### 5. Add GitHub repo secrets

In the specify repo on GitHub (**Settings** > **Secrets and variables** >
**Actions**), add:

| Secret                   | Value                                                        |
| ------------------------ | ------------------------------------------------------------ |
| `CLOUDFLARE_API_TOKEN`   | A Cloudflare API token with **Cloudflare Pages: Edit** permission |
| `CLOUDFLARE_ACCOUNT_ID`  | Your Cloudflare account ID (visible on the dashboard overview page) |

## Verification

After pushing to `main` with changes in `docs/`:

1. Check the **Actions** tab — the Docs workflow should build and deploy.
2. Visit `https://specify-docs.pages.dev` — you should be redirected to
   GitHub OAuth.
3. After authenticating with a GitHub account that belongs to the `augentic`
   org, the Specify Operator Guide should load.
