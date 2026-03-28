import request from "supertest";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  githubWebhookSignatureProfile,
  signWebhookPayload,
  verifyGitHubWebhookSignature,
} from "../src/webhooks/signatureVerification";

const secret = "github-webhook-secret";

function createPayloadBuffer(payload: unknown): Buffer {
  return Buffer.from(JSON.stringify(payload), "utf8");
}

function createGitHubSignature(payload: Buffer): string {
  return signWebhookPayload({
    payload,
    secret,
    algorithm: githubWebhookSignatureProfile.algorithm,
    prefix: githubWebhookSignatureProfile.prefix,
  });
}

describe("GitHub webhook signature verification", () => {
  it("accepts a valid GitHub signature", () => {
    const payload = createPayloadBuffer({ action: "opened", repository: { full_name: "owner/repo" } });

    expect(() =>
      verifyGitHubWebhookSignature({
        payload,
        secret,
        signatureHeader: createGitHubSignature(payload),
      }),
    ).not.toThrow();
  });

  it("rejects a missing signature", () => {
    const payload = createPayloadBuffer({ action: "opened" });

    expect(() =>
      verifyGitHubWebhookSignature({
        payload,
        secret,
        signatureHeader: undefined,
      }),
    ).toThrow(/Missing GitHub webhook signature/i);
  });

  it("rejects an invalid signature", () => {
    const payload = createPayloadBuffer({ action: "opened" });

    expect(() =>
      verifyGitHubWebhookSignature({
        payload,
        secret,
        signatureHeader: "sha256=deadbeef",
      }),
    ).toThrow(/Invalid GitHub webhook signature/i);
  });
});

describe("POST /api/webhooks/github", () => {
  beforeEach(() => {
    process.env.GITHUB_WEBHOOK_SECRET = secret;
    vi.resetModules();
  });

  async function getApp() {
    const { app } = await import("../src/app");
    return app;
  }

  it("rejects requests without a signature", async () => {
    const app = await getApp();

    await request(app)
      .post("/api/webhooks/github")
      .set("Content-Type", "application/json")
      .send(JSON.stringify({ action: "opened" }))
      .expect(401);
  });

  it("rejects requests with an invalid signature", async () => {
    const app = await getApp();

    await request(app)
      .post("/api/webhooks/github")
      .set("Content-Type", "application/json")
      .set("X-Hub-Signature-256", "sha256=deadbeef")
      .send(JSON.stringify({ action: "opened" }))
      .expect(401);
  });

  it("accepts requests with a valid signature", async () => {
    const app = await getApp();
    const rawPayload = JSON.stringify({
      action: "opened",
      number: 42,
      repository: { full_name: "owner/repo" },
      pull_request: { html_url: "https://github.com/owner/repo/pull/42" },
    });
    const signature = createGitHubSignature(Buffer.from(rawPayload, "utf8"));

    const res = await request(app)
      .post("/api/webhooks/github")
      .set("Content-Type", "application/json")
      .set("X-Hub-Signature-256", signature)
      .send(rawPayload)
      .expect(202);

    expect(res.body).toEqual({
      data: {
        authenticated: true,
        provider: "github",
        received: true,
      },
    });
  });
});
