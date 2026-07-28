# roadmap/phase_3.md

## Phase 3 — Open Agent Economy Substrate

**Theme:** Harden Aether into a substrate others build on: portable reputation, richer verification, multi-agent markets, and operational maturity — without abandoning infrastructure-first discipline.

**Entry:** Phase 2 exit criteria met; economic parameters stable enough for external builders.

## Objectives

1. Stabilize identity + payment APIs for external agent frameworks  
2. Offer pluggable verification backends (TEE/ZK/optimistic) behind common attestation interface  
3. Make reputation queries a reliable constraint surface for capabilities  
4. Support recursive “agent hires agent” escrow composition at meaningful volume  
5. Operationalize security response, parameter governance, and light-client tooling  

## Scope In

- Attestation registry with multiple proof kinds and clear assurance levels  
- Reputation threshold credentials / selective disclosure for counterparty gates  
- Channel network improvements (hubs or routing — based on Phase 2 evidence)  
- Builder documentation, SDKs, and conformance tests  
- Performance and cost SLOs published (p50/p95)  
- Optional regulated credential bridges (application-optional)  

## Scope Out

- Becoming an LLM product company  
- Guaranteeing agent cognitive alignment  
- Speculative consumer token campaigns as success criteria  

## Exit Criteria

- [ ] Independent teams run agents against Aether without custom protocol forks  
- [ ] Measured microsettlement SLOs met under load tests  
- [ ] At least two attestation backends usable in escrow flows  
- [ ] Reputation-gated capabilities used in production-like markets  
- [ ] Incident response and upgrade playbooks exercised in drills  
- [ ] Honest public reporting: what is soft finality vs hard finality  

## Key Risks

| Risk | Mitigation |
|------|------------|
| Substrate captured by speculative narrative | Non-goals + utility metrics in public comms |
| Verification backend fragmentation | Strict type registry + conformance tests |
| Systemic reputation cascades | Diversity metrics, bond scaling, circuit breakers |

## Success Metrics (Illustrative)

- Autonomous loops completed without HITL  
- Cost per 1k micro-settlements  
- Dispute rate and wrongful release rate  
- Time from register→first bonded job for new agents  
- External builder retention  

## Long Horizon

Beyond Phase 3, Aether should be evaluated as **default economic plumbing for autonomous software** — not as a ticker. Further phases (if any) should be driven by measured bottlenecks in identity, settlement, proof cost, or reputation adversaries.

## Links

- [VISION.md](../VISION.md)
- [WHITEPAPER.md](../WHITEPAPER.md)
- [phase_1.md](phase_1.md) · [phase_2.md](phase_2.md)
