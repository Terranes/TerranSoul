# Tutorial: Share a TerranSoul Brain on Your LAN

This tutorial walks through a local-network setup where Alice hosts a
TerranSoul MCP brain on her desktop and other TerranSoul devices on the same
LAN connect to retrieve query-scoped memory results.

The example story is Alice learning Vietnamese laws: Alice has imported her own
Vietnamese legal notes and approved reference documents into TerranSoul, then
lets nearby TerranSoul clients search that knowledge without copying her whole
database.

> Legal note: this is a software workflow tutorial, not legal advice. Use
> official sources and qualified legal review for real legal decisions.

## What You Are Building

```text
Alice's desktop TerranSoul
  MCP HTTP server: 0.0.0.0:7421 when LAN mode is enabled
  LAN discovery: UDP 7424 broadcast, no token in broadcast
  Brain data: Alice's local SQLite memory/RAG store
        |
        | bearer-token MCP search over the same Wi-Fi/LAN
        v
Bob's TerranSoul / Mai's TerranSoul
  Discover Alice, paste token, run remote brain_search queries
```

Remote clients retrieve ranked memory snippets through MCP. They do not receive
Alice's whole `memory.db`, and discovery does not broadcast the bearer token.

## Requirements

- Two or more TerranSoul desktop apps on the same trusted LAN.
- Alice's TerranSoul has a configured brain and the documents she wants to
  share already ingested into memory.
- The MCP server is running on Alice's machine.
- Windows/macOS/Linux firewall allows TerranSoul on the MCP TCP port and UDP
  discovery port `7424`.
- Alice shares the bearer token out-of-band, for example in person or through a
  secure company channel.

## Step 1: Alice Teaches Her Local Brain

Alice first imports the knowledge she wants TerranSoul to retrieve. For this
example, she stores notes such as:

- Vietnamese labor-law summaries she wrote herself.
- Official documents or PDFs she is allowed to store locally.
- Her own question-and-answer notes about contracts, probation, overtime, and
  social insurance.

She can use the normal Memory/Brain ingestion flow, chat-based teach flow, or an
approved MCP `brain_ingest_url` call. Keep source URLs and tags clear, for
example:

```text
tags: vietnamese-law,labor,alice-notes
importance: 4
```

## Step 2: Alice Enables LAN Mode

On Alice's desktop:

1. Open **Brain**.
2. Open **LAN Brain Sharing**.
3. Turn on **Enable LAN brain sharing and discovery on this device**.

This setting is intentionally off by default. TerranSoul reads it when the MCP
server starts, so enable it before starting MCP. If MCP was already running,
stop and start it again so it rebinds for LAN access.

## Step 3: Alice Starts the MCP Server

Still on Alice's desktop:

1. In **AI Coding Integrations**, start the MCP server.
2. Confirm it reports a running port such as `7421`.

The same server powers LAN retrieval. With LAN mode enabled before startup,
TerranSoul binds the server to LAN interfaces instead of loopback-only.

## Step 4: Alice Starts Sharing Her Brain

Still on Alice's desktop:

1. In **Share Your Brain**, enter a name such as `Alice - Vietnamese law notes`.
2. Click **Start Sharing**.
3. Click **Copy** next to the token and share it only with trusted devices.

![Alice starts sharing her Vietnamese law notes](screenshots/lan-mcp-alice-host.svg)

What happens under the hood:

- TerranSoul advertises Alice's brain name, host, provider, memory count, and
  read-only status via UDP `7424`.
- The bearer token is not included in that discovery packet.
- Other clients must still authenticate to Alice's MCP HTTP endpoint with the
  copied token.

## Step 5: Bob Discovers Alice's Brain

On Bob's TerranSoul desktop:

1. Open **Brain -> LAN Brain Sharing**.
2. Enable LAN brain sharing and discovery on this device.
3. In **Discover Shared Brains**, click **Scan Network**.
4. Bob should see `Alice - Vietnamese law notes` with Alice's LAN address and
   memory count.
5. Click **Connect** to copy Alice's host and port into the manual form.

![Bob scans the LAN and finds Alice's brain](screenshots/lan-mcp-client-discover.svg)

If discovery fails, Bob can still connect manually with Alice's LAN IP, MCP
port, and token.

## Step 6: Bob Connects with Alice's Token

Bob enters the token Alice shared out-of-band:

1. Check the **Manual Connect** form.
2. Confirm `Host` is Alice's LAN IP, for example `192.168.1.42`.
3. Confirm `Port` is Alice's MCP port, for example `7421`.
4. Paste the token.
5. Click **Connect**.

![Bob connects to Alice's remote brain](screenshots/lan-mcp-client-connect.svg)

TerranSoul validates the connection by calling Alice's remote `/health` route.
After success, Bob sees Alice under **Connected Brains**.

## Step 7: Bob Retrieves Vietnamese Law Context

Now Bob can ask Alice's brain for targeted context without copying her whole
memory database.

Example query:

```text
What should I check before signing a labor contract in Vietnam?
```

In **Search All Connected Brains**, Bob enters the query and clicks **Search**.
TerranSoul sends an authenticated MCP `brain_search` request to Alice's machine,
then shows scored snippets tagged with Alice's brain name.

![Bob retrieves legal context from Alice's brain](screenshots/lan-mcp-remote-search.svg)

Good retrieval results should show:

- Source brain name, such as `Alice - Vietnamese law notes`.
- A relevance score.
- Snippets from Alice's stored notes or documents.
- Tags such as `vietnamese-law,labor,contracts`.

## Security Checklist

- Use this only on a trusted LAN. Do not enable it on airport, cafe, hotel, or
  public Wi-Fi.
- Share the token out-of-band. Discovery never broadcasts it, and neither
  should you.
- Regenerate the MCP token if it is pasted in the wrong place.
- Stop sharing when the session is over.
- Keep sensitive personal, legal, or company data in a separate brain/profile if
  not every connected user should see it.
- Remote retrieval is query-scoped, but a trusted client can still ask many
  queries. Treat token access as read access to the shared knowledge surface.

## Troubleshooting

| Symptom | What To Check |
|---|---|
| `LAN mode not enabled` | Enable LAN brain sharing and discovery in the Brain view before starting sharing. |
| `MCP server must be running` | Start the MCP server in **AI Coding Integrations** before **Start Sharing**. |
| Bob cannot discover Alice | Confirm both devices are on the same subnet and UDP `7424` is not blocked. |
| Manual connect fails | Check Alice's LAN IP, MCP port, and token; confirm Alice's app is still running. |
| Search returns empty results | Alice may need to ingest/tag documents first, or Bob's query may need more specific terms. |
| Browser/Vercel client cannot connect | Use native desktop/mobile pairing for reliable LAN access; public HTTPS browser pages cannot receive UDP discovery packets. |

## Mental Model

Think of LAN sharing as a temporary read window into another local TerranSoul
brain:

- Alice owns and stores the data.
- Bob owns the question.
- MCP carries the authenticated query.
- TerranSoul returns only ranked context snippets.

That makes it useful for household knowledge, company policy, team research,
and Alice's Vietnamese law study notes without turning every device into a copy
of Alice's database.