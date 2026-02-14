---
name: gog
description: Google Workspace CLI for Gmail, Calendar, Drive, Contacts, Sheets, and Docs.
---

# gog — Google Workspace CLI

You have access to `gog`, a CLI tool for the full Google Workspace suite: Gmail, Calendar, Drive, Docs, Sheets, Slides, Contacts, Tasks, and Chat. Use the `exec` tool to run gog commands.

## Accounts

Google accounts are configured during setup. Use `gog accounts list` to see available accounts.
- The **default account** is used when no `--account` flag is provided
- Additional accounts can be targeted with `--account user@example.com`

## Calendar Commands

```bash
# List upcoming events (default account)
gog calendar list

# List events for a date range
gog calendar events primary --from 2026-02-08T00:00:00Z --to 2026-02-15T00:00:00Z

# List events for a specific account
gog calendar events primary --from 2026-02-08T00:00:00Z --to 2026-02-15T00:00:00Z --account user@example.com

# Create an event
gog calendar create primary --summary "Meeting title" --from 2026-02-10T14:00:00Z --to 2026-02-10T15:00:00Z

# Update an event
gog calendar update primary <eventId> --summary "New Title"

# Show calendar colors
gog calendar colors
```

## Gmail Commands

```bash
# Search recent emails
gog gmail search 'newer_than:7d' --max 10

# Search specific sender
gog gmail messages search "in:inbox from:someone@example.com" --max 20

# Send plain text email
gog gmail send --to recipient@example.com --subject "Subject" --body "Message body"

# Send multi-line email via heredoc
gog gmail send --to recipient@example.com --subject "Subject" --body-file - <<'EOF'
Hi,

This is the message body.

Best regards
EOF

# Reply to a message
gog gmail send --to recipient@example.com --subject "Re: Original" --body "Reply text" --reply-to-message-id <msgId>

# Create draft
gog gmail drafts create --to recipient@example.com --subject "Draft" --body "Content"
```

## Google Docs Commands

```bash
# Create a new Google Doc
gog docs create "Document Title"

# Create in a specific account
gog docs create "Document Title" --account user@example.com

# Get doc info
gog docs info <docId>

# Read doc content as plain text
gog docs cat <docId>

# Export doc (pdf, docx, txt)
gog docs export <docId> --format pdf

# Copy a doc
gog docs copy <docId> "Copy Title"
```

## Google Sheets Commands

```bash
# Create a new spreadsheet
gog sheets create "Sheet Title"

# Read a range
gog sheets get <spreadsheetId> "Sheet1!A1:D10"

# Update cells
gog sheets update <spreadsheetId> "Sheet1!A1" "value1" "value2" "value3"

# Append rows
gog sheets append <spreadsheetId> "Sheet1!A:D" "col1" "col2" "col3" "col4"

# Clear a range
gog sheets clear <spreadsheetId> "Sheet1!A1:D10"

# Get spreadsheet metadata (sheet names, etc)
gog sheets metadata <spreadsheetId>

# Export (pdf, xlsx, csv)
gog sheets export <spreadsheetId> --format csv
```

## Google Slides Commands

```bash
# Create a new presentation
gog slides create "Presentation Title"

# Get presentation info
gog slides info <presentationId>

# Export (pdf, pptx)
gog slides export <presentationId> --format pdf

# Copy a presentation
gog slides copy <presentationId> "Copy Title"
```

## Google Drive Commands

```bash
# List files in root
gog drive ls

# List files in a folder
gog drive ls --parent <folderId>

# Search files
gog drive search "quarterly report"

# Create a folder
gog drive mkdir "Folder Name"

# Upload a file
gog drive upload /path/to/file.pdf

# Download a file
gog drive download <fileId>

# Move a file
gog drive move <fileId> --to <folderId>

# Rename a file
gog drive rename <fileId> "New Name"

# Get file info
gog drive get <fileId>

# Get file URL
gog drive url <fileId>
```

## Important Rules

- Always confirm with the user before sending emails or creating/modifying calendar events
- When the user specifies which Google account to use, pass `--account <email>`
- Use `--json` flag when you need to parse output programmatically
- Use `gog accounts list` to see which accounts are configured
- Use `primary` as the calendarId for the main calendar
- After creating a Doc/Sheet/Slides, use `gog drive url <id>` to get the shareable link
- For multi-step document creation (e.g., creating a doc then populating it), create first then use the returned ID for subsequent operations
