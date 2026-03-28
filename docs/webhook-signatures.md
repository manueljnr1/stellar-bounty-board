# Webhook Signature Verification

Use the backend webhook signature utility whenever a third-party service sends data into the API. This keeps webhook routes reusable and prevents unauthenticated traffic from reaching integration logic.

## What exists today

- `backend/src/webhooks/signatureVerification.ts` provides a generic HMAC signature verifier.
- The same module exports GitHub defaults for `X-Hub-Signature-256` and `sha256=...` signatures.
- `/api/webhooks/github` is protected by the middleware and can be extended with GitHub event handling later.

## GitHub usage

Set `GITHUB_WEBHOOK_SECRET` in the backend environment, then protect the route with the GitHub middleware:

```ts
app.post(
  "/api/webhooks/github",
  createGitHubWebhookSignatureMiddleware(() => process.env.GITHUB_WEBHOOK_SECRET),
  handler,
);
```

The middleware verifies the raw request body against the `X-Hub-Signature-256` header and returns `401` when the signature is missing or invalid.

## For future integrations

- Reuse `createWebhookSignatureMiddleware(...)` for any provider that signs requests with an HMAC header.
- Keep raw body capture enabled so verification uses the original bytes sent by the provider.
- Add provider-specific wrappers when a new integration has a stable header name, prefix, and algorithm.
- Reject unsigned traffic before parsing event-specific fields or mutating any application state.
