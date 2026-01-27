# Script for Roadmap for Hermes

## Contract Language

The language in our contract is as follows:

This workstream is dedicated to replacing the existing federated side-chain infrastructure with a fully distributed,
immutable ledger and peer-to-peer architecture.

Initial development will focus on the core components necessary to achieve this, including:

* Upgrade of the WebAssembly (“WASM”) Engine to the latest WASM Component Model.
* Enhancements to the WASM module Linker for partial linking.
* Development of event handling for WASM-driven validation of data for
  example published over InterPlanetary File System
  (“IPFS”) Publish/Subscribe or Distributed Hash Table (“DHT”).
* Setting up the framework for parallel WASM module execution for future performance and scalability testing.
* Initial design and implementation of a generalized solution for uniform system resource management.
* Enabling Hermes packages to read data directly from IPFS.
* Developing the capability for Hermes applications to execute from an IPFS link.
* Commencing implementation of cryptography for applied research into Quadratic Voting and Time-Weighted Stake models.

## Community Proposal Language

The language in our Proposal to the community for this work is as follows:

Production-Grade Decentralized Catalyst Infrastructure via Hermes

Objective: Advance the decentralization, scalability, and auditability of Project Catalyst by
delivering a rigorously stress tested implementation of Hermes to replace federated infrastructure
with a fully distributed, peer-to-peer system.
This includes enabling parallel voting events, secure Cardano-based vote casting, and public auditability of
historic voting data that eliminates reliance on Web2 infrastructure services, empowering open innovation
and ecosystem governance.
Outcome: maturing the state of the art of the Project Catalyst technology stack beyond the existing
proof of concept and delivering production-ready Catalyst infrastructure using a fully distributed
database and immutable ledger, configurable administration interfaces, eliminating reliance on a
federated side-chain and small number of nodes while maintaining many artifacts are published to
Cardano main-chain.

Output: Enhanced scalability and flexibility of Catalyst governance:
multiple funding rounds using multiple tokens can run concurrently or overlap, dramatically
increasing the system's utility and responsiveness.
Stronger security and voter confidence through direct blockchain-based verification and vote casting,
reducing trust assumptions.
Full decentralization of Catalyst infrastructure : no dependency on federated servers or Web2
storage, reducing censorship risks and increasing resilience.
Greater transparency and auditability : historic voting data is verifiable, immutable,
and accessible through distributed networks Developer empowerment :
Builders can deploy secure, complex, and custom applications like governance mechanisms on Hermes
with minimal barriers via IPFS and WebAssembly.

Features:

* Upgrade WASM Engine to latest Wasm Component Model required for Hermes Application Logic.
* Enhance WASM module Linker to support partial linking for modules which do not contain all events,
  or use all functions provided by the Hermes runtime.
* Add events for WASM driven validation of data published over IPFS Pub/Sub or the DHT.
* Make Hermes engine execute multiple WASM modules in parallel for performance and scalability testing
* Implement a generalized solution to uniformly manage system resources.
* Hermes package can read data directly from IPFS, not from a local copy downloaded from IPFS.
* Enable execution of Hermes applications from an IPFS link, not only a locally present application.
* Implement cryptography for 2024’s applied research into Quadratic Voting and Time-Weighted Stake models

The Roadmap is as follows (all dates are approximate):

* Start Jan 2026, Duration 4 weeks, Wasm Engine Component Model Upgrade
* Mid Jan 2026, Duration 6 weeks, Wasm Module Linker Upgrade
* Start Feb 2026, Duration 8 weeks, Event-Driven Data Validation on IPFS
* Start March 2026, Duration 4 weeks, Parallel Wasm Module Execution Framework
* Mid March 2026, Duration 6 weeks, Uniform Resource Management
* Start April 2026, Duration 6 weeks, IPFS Direct Data Readability
* Start May 2026, Duration 4 weeks, Execution From IPFS Link
