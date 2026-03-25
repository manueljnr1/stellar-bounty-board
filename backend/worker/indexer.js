// Soroban Contract Event Indexer Worker
// Polls contract events and normalizes them for backend use

import axios from "axios";
import fs from "fs";
import path from "path";

// CONFIGURATION
const CONTRACT_ID = process.env.SOROBAN_CONTRACT_ID || ""; // Set in env
const SOROBAN_RPC_URL = process.env.SOROBAN_RPC_URL || "https://rpc-futurenet.stellar.org";
const POLL_INTERVAL_MS = 10000; // 10 seconds
const INDEX_FILE = path.join(__dirname, "indexed-events.json");

// Event normalization mapping
function normalizeEvent(event) {
  // Example: map Soroban event to backend-friendly record
  // Adjust mapping as contract evolves
  return {
    id: event.id,
    type: event.type, // create, reserve, release, refund
    bountyId: event.bounty_id,
    actor: event.actor,
    timestamp: event.timestamp,
    raw: event,
  };
}

// Save events to file (or replace with DB logic)
function saveEvents(events) {
  fs.writeFileSync(INDEX_FILE, JSON.stringify(events, null, 2));
}

// Load last indexed event (for polling)
function loadLastEventId() {
  if (!fs.existsSync(INDEX_FILE)) return null;
  const events = JSON.parse(fs.readFileSync(INDEX_FILE, "utf-8"));
  return events.length ? events[events.length - 1].id : null;
}

// Poll Soroban contract events
async function pollEvents() {
  let lastEventId = loadLastEventId();
  try {
    // Replace with actual Soroban event fetch logic
    const res = await axios.get(`${SOROBAN_RPC_URL}/events`, {
      params: {
        contract_id: CONTRACT_ID,
        from_id: lastEventId,
      },
    });
    const events = res.data.events || [];
    if (events.length) {
      const normalized = events.map(normalizeEvent);
      let allEvents = [];
      if (fs.existsSync(INDEX_FILE)) {
        allEvents = JSON.parse(fs.readFileSync(INDEX_FILE, "utf-8"));
      }
      allEvents.push(...normalized);
      saveEvents(allEvents);
      console.log(`[Indexer] Indexed ${events.length} new events.`);
    } else {
      console.log("[Indexer] No new events.");
    }
  } catch (err) {
    console.error("[Indexer] Polling error:", err.message);
  }
}

function startWorker() {
  console.log("[Indexer] Starting Soroban contract event indexer...");
  setInterval(pollEvents, POLL_INTERVAL_MS);
}

if (require.main === module) {
  startWorker();
}

export { pollEvents, startWorker };
