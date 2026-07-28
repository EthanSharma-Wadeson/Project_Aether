# experiments/prototypes.md

## Purpose

Track prototypes that test hypotheses and de-risk architecture choices. Prefer thin vertical slices over platform theater.

## Naming Convention

**Prototypes** use `PROTO-N` identifiers. **Project phases** use `Phase N`. Do not use `P0`–`P5` — they collide with phase numbers.

| ID | Name |
|----|------|
| PROTO-0 | Identity and Capability Prototype |
| PROTO-1 | Payment Channel Prototype |
| PROTO-2 | Escrow Prototype |
| PROTO-3 | Reputation Indexer Prototype |
| PROTO-4 | Settlement Backend Spike |
| PROTO-5 | Selective Disclosure Credential Demo (stretch) |

See [Project_Phases/phase_0/CONTEXT.md](../../Project_Phases/phase_0/CONTEXT.md) for the full phase vs prototype distinction.

## Prototype Principles

- Measure latency, cost, and failure modes  
- Headless clients only (no human wallet UI as the happy path)  
- Log soft vs hard finality explicitly  
- Destroy prototypes freely; graduate ideas into specs  

## PROTO-0 — Identity and Capability Prototype

**Goal:** Register agent identities, issue spend-limited capabilities, reject over-limit actions.

**Exercises:** H2  

**Deliverables:** Local library + tests; message schemas draft  

**Status:** Planned  

## PROTO-1 — Payment Channel Prototype

**Goal:** Open/update/settle/dispute in a simulated clock; inject crash and equivocation.

**Exercises:** H1, H6  

**Deliverables:** State machine reference; metrics harness  

**Status:** Planned  

## PROTO-2 — Escrow Prototype

**Goal:** Consumer agent hires provider agent; payment releases on receipt verify or timeout refund.

**Exercises:** H4, H5  

**Deliverables:** End-to-end script; failure injection  

**Status:** Planned  

## PROTO-3 — Reputation Indexer Prototype

**Goal:** Ingest settlement/dispute events; compute score vector; run wash-trade adversary.

**Exercises:** H3  

**Deliverables:** Attack ROI report; scorer v0  

**Status:** Planned  

## PROTO-4 — Settlement Backend Spike

**Goal:** Implement certificate posting / channel settle on 1–2 candidate bases; compare p95 cost/latency.

**Exercises:** H1, H5; feeds `research/blockchain_analysis.md`  

**Deliverables:** Comparison table; recommendation memo  

**Status:** Planned  

## PROTO-5 — Selective Disclosure Credential Demo (stretch)

**Goal:** Prove bond threshold or reputation threshold without revealing full evidence set.

**Exercises:** Privacy model feasibility  

**Deliverables:** Demo + cost notes  

**Status:** Planned (post PROTO-3)  

## Harness Requirements

- Deterministic simulation mode for CI  
- Adversary plugs: crash, delay, equivocate, wash  
- Metrics export (JSON) for comparison across spikes  

## Graduation Checklist

A prototype graduates when:

- [ ] Metrics recorded against its hypotheses  
- [ ] Security footguns written down  
- [ ] Spec PRs opened against PROTOCOL / subsystem docs  
- [ ] Code either archived or moved to an implementation repo  

## Results Index

| Prototype | Link / commit | Summary |
|-----------|---------------|---------|
| — | — | — |
