# Sv2 Protocol Coverage

This document formally describes how `interoperability-tests-sv2` covers the [Stratum V2 Protocol Specification](https://github.com/stratum-mining/sv2-spec).

§ indicates spec sections.

## Scenarios

A **Scenario** is a single testable Sv2 Protocol behavior — each describes a failure mode, its spec basis, and how a client detects it.

Scenarios are application-agnostic: each row describes a generic Sv2 Protocol behavior, which might potentially happen under different application contexts.

### How scenarios are organized

The catalog is organized **one table per spec message**, mirroring the spec's own structure. A **General** section covers cross-cutting invariants that have no single home message (framing, error codes, channel/job namespaces, liveness).

**Anchor rule:** each scenario is filed under the message whose spec section carries the normative requirement it tests. Scenarios frequently involve a longer session with other messages; the `Involves` column lists them so the scenario remains discoverable from every message it touches.

**Symmetric split**: scenarios that apply to both a standard and extended message pair are separate scenarios with separate IDs — they exercise different server code paths and are covered independently per suite.

**One failure mode per scenario**: a row describes a single way a server can fail. Failures that have different spec bases, or that a server can exhibit independently of each other, are separate scenarios even when one probe could exercise both.

**Scenario IDs** are `<PROTO>-<MSG>-<n>` where `PROTO` ∈ `GEN` (cross-cutting), `C` (common), `M` (mining), `JD` (job declaration), `TD` (template distribution). Extension scenarios use `X<extid>-<MSG>-<n>` (e.g. `X0001-REQS-1` for extension `0x0001`). IDs are unique, stable references: new scenarios append; existing IDs are never renumbered or reused. IDs prefixed `JD-`/`TD-` are reserved for future suite expansion.

## Suites

A **Suite** targets one protocol role defined in [§3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#3-protocol-overview) and covers the scenarios an implementation of that role must pass. Deployments map onto roles: a pool, a solo pool, a proxy and a Job Declarator Client all act as a **Mining Protocol Server** toward their downstream. Scenarios that only apply when the target uses an optional feature (group channels, extensions, proxy forwarding) say so in their row instead of living in a separate suite. The [**Suite Coverage**](#suite-coverage) table below tracks which scenarios are implemented per suite.

## Compliance Tiers

**Compliance Tiers** classify scenarios by how severely a failure undermines Sv2 compliance.

- **Ω1 — non-compliant.** An application failing any Ω1 scenario is fundamentally broken and does not meet the Sv2 protocol specification.
- **Ω2 — interoperability-critical.** Meaningful edge cases that might hurt interoperability, but are not direct protocol violations.
- **Ω3 — robustness.** Miscellaneous non-critical corner cases.

**Assignment rule.** A scenario's tier follows the spec text behind it.

- **Ω1** when the spec states the requirement on the server. That covers a MUST / MUST NOT addressed to the server, and an unambiguous statement of server behavior, such as "will be rejected", "can be sent only to", or a field defined as echoing the request.
- **Ω2** when the spec only says SHOULD / SHOULD NOT, or when the requirement is derived from other text without being stated. The `Spec basis` column then says *derived*. A MUST addressed to the client lands here when the scenario tests that the server rejects input violating it: a server that tolerates the violation breaks no obligation of its own.
- **Ω3** when no spec text exists and the scenario captures expected behavior only.

This list might grow as the project matures.

## Testability Tiers

**Testability Tiers** classify scenarios by the type of assertion required.

- **Θ1 — Protocol conformance.** Pure message-level assertions: framing, negotiation, channel open, message ordering, invalid-share rejection, channel lifecycle.
- **Θ2 — Valid-share accounting.** The test client grinds real shares meeting the *channel* target and asserts acceptance / rejection and batch-accounting semantics. Every scenario whose failure mode is a share being *accepted* is Θ2: a random hash gets `SubmitShares.Error` from any server, so acceptance is only observable with an otherwise-valid share.

Some scenarios depend on something only the server decides: sending `SetExtranoncePrefix` or `SetGroupChannel`, closing a channel, setting a flag in `SetupConnection.Success`, engaging with extension negotiation. Others depend on the network, such as a new block arriving. The test client cannot provoke any of these; the `Detection` column says so, and a run in which the precondition never occurs reports the scenario as *not exercised*, never as passed.

This list might grow as the project matures.

---

## Scenario Catalog

### General ([§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping)–[§3.5](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#35-error-codes), [§5.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#51-job)–[§5.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#52-channel), [§8](https://github.com/stratum-mining/sv2-spec/blob/main/08-Message-Types.md#8-message-types))

Cross-cutting invariants with no single home message.

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| GEN-1 | Ω1 | Θ1 | `channel_msg` bit wrong per message type (e.g. `NewMiningJob` 0x15 sent with bit unset, `OpenStandardMiningChannel.Success` 0x11 with bit set) | [§3.2.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#321-routing-frames-over-channels) "when set, indicates a message which is specific to a channel, whereas if it is unset, the message is to be interpreted by the immediate receiving device", [§8](https://github.com/stratum-mining/sv2-spec/blob/main/08-Message-Types.md#8-message-types) | Assert bit on every received frame | all messages |
| GEN-2 | Ω1 | Θ1 | Core messages sent with non-zero `extension_type` | [§3.4.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#341-extension-type-field-usage) "MUST have `extension_type` set to `0x0000`" | Assert `extension_type & 0x7FFF == 0x0000` (mask `channel_msg` bit first) | all messages |
| GEN-3 | Ω1 | Θ1 | Server-sent error codes containing bytes outside printable ASCII (control characters or non-printable bytes) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `STR0_255` "Every byte MUST be a printable ASCII character (0x20 to 0x7E). A receiver MUST treat any other byte as a protocol error." | Validate every `error_code` string; assert printable ASCII only | all `*.Error` messages |
| GEN-4 | Ω2 | Θ1 | Server sends messages for a `channel_id` never opened on this connection | [§5.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#52-channel) (channel ID consistency) (derived) | Track open channel set; assert all inbound channel messages reference it | all channel messages |
| GEN-5 | Ω2 | Θ1 | Job starvation: Mining Protocol Server never sends fresh jobs after the first, stranding miners on an exhausted search space | [§5.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#51-job) (server must manage hashing space and "provide new and unique Jobs quickly enough") (derived) | Soft liveness/timing metric | `NewMiningJob`, `NewExtendedMiningJob` |
| GEN-6 | Ω1 | Θ1 | Message with unknown `extension_type` that terminates locally is not discarded and ignored (server errors, hangs up, or crashes) | [§3.4.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#341-extension-type-field-usage) "Messages with an unknown `extension_type` which are to be processed locally (as defined above) MUST be discarded and ignored" | Send a frame with an unallocated `extension_type`; assert the session continues unaffected | all messages |
| GEN-7 | Ω2 | Θ2 | Server chokes on unnegotiated/unknown TLV fields appended to a core message | [§3.4.3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#343-stratum-v2-tlv-encoding-model) ("unknown fields can be safely ignored") (derived) | Grind a valid share; append an unknown TLV to its `SubmitSharesExtended`; assert the share is processed normally | `SubmitSharesExtended`, `SubmitShares.Success` |
| GEN-8 | Ω1 | Θ1 | Server-generated TLV fields violate placement/ordering rules (not at end of payload, not ordered by `extension_type`) | [§3.4.3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#343-stratum-v2-tlv-encoding-model) "TLV fields MUST be placed at the end of the message payload" and "TLV fields MUST be ordered by `extension_type`" | Parse TLV region on server messages (only if the server negotiates a TLV extension) | all TLV-extended messages |
| GEN-9 | Ω1 | Θ2 | TLV field exceeding its extension's maximum length is accepted | [§3.4.3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#343-stratum-v2-tlv-encoding-model) "the recipient MUST reject the message" | Negotiate ext `0x0002`; submit an otherwise-valid share (meets the channel target) carrying a 33-byte printable `user_identity`; assert `SubmitShares.Error` (only if the server supports extension `0x0002`) | `RequestExtensions`, `SubmitSharesExtended`, `SubmitShares.Error` |
| GEN-10 | Ω1 | Θ1 | Proxy drops an unknown-extension channel message, or modifies any byte other than the permitted `channel_id` remap, instead of forwarding it to the channel endpoint | [§3.4.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#341-extension-type-field-usage) "MUST forward that message... Any `channel_id` mapping/conversion required for other channel messages MUST be done on the `channel_id` in the first four bytes of the message payload, but the message MUST NOT be otherwise modified" | Inject an unknown-`extension_type` channel message on one side of the proxy; assert it is forwarded with only the first four payload bytes allowed to differ (only if the target is a proxy, with the harness on both sides of it) | all channel messages |
| GEN-11 | Ω1 | Θ1 | Server sends extension-specific messages for an extension that was never negotiated | [§3.4.2](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#342-implementing-extensions-support) "Extensions MUST require negotiation... before sending" | Assert no extension-typed frames other than `RequestExtensions.*` arrive without a prior `RequestExtensions.Success` | `RequestExtensions.Success` |
| GEN-12 | Ω2 | Θ1 | Server sends contradictory responses to a single request (e.g. both `.Success` and `.Error` for the same `request_id`) | `request_id` definitions in [§5.3.3](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#533-openstandardminingchannelsuccess-server---client), [§5.3.5](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#535-openextendedminingchannelsuccess-server---client), [§5.3.6](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#536-openminingchannelerror-server---client), ext `0x0001` [§2](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#2-messages-defined-by-this-extension) (derived) | Assert at most one response per `request_id`; zero responses are handled by the message-specific silence scenarios and are compliant for `RequestExtensions` when the server does not engage | `OpenMiningChannel.Error`, `RequestExtensions.Error` |
| GEN-15 | Ω2 | Θ1 | Server sends a `B0_32` field longer than 32 bytes | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `B0_32` "Allowed range of length is 0 to 32" (derived: field type range) | Validate the length prefix on every received `B0_32` field (notably `extranonce_prefix` in `OpenStandardMiningChannel.Success`, `OpenExtendedMiningChannel.Success`, and `SetExtranoncePrefix`); assert ≤ 32 | `OpenStandardMiningChannel.Success`, `OpenExtendedMiningChannel.Success`, `SetExtranoncePrefix` |

### Common Protocol Messages

#### `SetupConnection` ([§3.6.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#361-setupconnection-client---server), mining flags [§5.3.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#531-setupconnection-flags-for-mining-protocol))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| C-SC-1 | Ω1 | Θ1 | Server sends *any* protocol message before responding to `SetupConnection` | [§3.6.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#361-setupconnection-client---server) "The server MUST NOT send any message before its `SetupConnection` response" | Message-order assertion | `SetupConnection.Success`, `SetupConnection.Error` |
| C-SC-2 | Ω2 | Θ1 | Server rejects a `SetupConnection` with an empty `device_id`, locking out clients that don't report telemetry | [§3.6.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#361-setupconnection-client---server) "Clients that are not configured to provide telemetry data to the upstream node SHOULD set `device_id` to 0-length strings" | Send `SetupConnection` with empty `device_id`; assert `.Success` | `SetupConnection.Success` |
| C-SC-3 | Ω1 | Θ1 | Server sets `REQUIRES_FIXED_VERSION` in `.Success.flags` even though client set `REQUIRES_VERSION_ROLLING` | [§5.3.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#531-setupconnection-flags-for-mining-protocol) "MUST NOT be set" | Flag cross-check | `SetupConnection.Success` |
| C-SC-4 | Ω2 | Θ1 | Server that serves Standard Channels (does not set `REQUIRES_EXTENDED_CHANNELS`) rejects a `SetupConnection` carrying `REQUIRES_STANDARD_JOBS` (bit 0) — fails the most basic client class | [§5.2.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#522-extended-channel) "MUST also accept Standard Channels unless they set `REQUIRES_EXTENDED_CHANNELS`" (derived: no sentence requires accepting `REQUIRES_STANDARD_JOBS`) | Send `SetupConnection` without bit 0; if `.Success.flags` lacks `REQUIRES_EXTENDED_CHANNELS`, a fresh connection with `REQUIRES_STANDARD_JOBS` must get `.Success` | `SetupConnection.Success`, `SetupConnection.Error` |
| C-SC-5 | Ω1 | Θ1 | Server sets `REQUIRES_EXTENDED_CHANNELS` in `.Success.flags` but then accepts `OpenStandardMiningChannel` instead of responding with `OpenMiningChannel.Error` | [§5.3.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#531-setupconnection-flags-for-mining-protocol) "If it does, the server MUST respond with `OpenMiningChannel.Error`" | After a `.Success` carrying `REQUIRES_EXTENDED_CHANNELS`, send `OpenStandardMiningChannel`; assert `OpenMiningChannel.Error` (only if the server sets `REQUIRES_EXTENDED_CHANNELS`) | `SetupConnection.Success`, `OpenStandardMiningChannel`, `OpenMiningChannel.Error` |
| C-SC-6 | Ω1 | Θ1 | Server sets `REQUIRES_FIXED_VERSION` but later sends `NewExtendedMiningJob` with `version_rolling_allowed = True` | [§5.3.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#531-setupconnection-flags-for-mining-protocol) "if this bit is set, extended jobs MUST NOT indicate support for version rolling" | Cross-message consistency (only if the server sets `REQUIRES_FIXED_VERSION`) | `NewExtendedMiningJob` |
| C-SC-7 | Ω1 | Θ1 | Server accepts `SetupConnection` with a `protocol` it does not serve | [§3.6.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#361-setupconnection-client---server) "If the server does not support the requested `protocol`, it MUST respond with `SetupConnection.Error`" | Send `SetupConnection` with a valid `protocol` the suite's target does not serve (e.g. `protocol=1`, Job Declaration, against a Mining Protocol Server); assert `SetupConnection.Error` | `SetupConnection.Error` |
| C-SC-8 | Ω1 | Θ1 | Server accepts `SetupConnection` with no supported version in the offered range (e.g. `(3,3)`, `(3,2)`, `(1,1)`) or closes without first sending `SetupConnection.Error` | [§3.6.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#361-setupconnection-client---server) "If the server supports no protocol version between `min_version` and `max_version` inclusive, it MUST respond with `SetupConnection.Error`" | Send version ranges with no overlap with 2; assert framed `SetupConnection.Error` before TCP close | `SetupConnection.Error` |
| C-SC-9 | Ω2 | Θ1 | Server crashes or drops the connection on 255-byte device-info strings (`vendor`, `hardware_version`, `firmware`, `device_id`) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `STR0_255` "Allowed range of length is 0 to 255" (derived) | Send `SetupConnection` with 255-byte printable-ASCII strings (so the probe exercises only the length bound); assert a framed `.Success` or `.Error` | `SetupConnection.Success`, `SetupConnection.Error` |
| C-SC-10 | Ω1 | Θ1 | Server accepts `SetupConnection` with a `protocol` value outside the ones the spec defines | [§3.6.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#361-setupconnection-client---server) "If the server does not support the requested `protocol`, it MUST respond with `SetupConnection.Error`" (`protocol` defines only 0, 1 and 2, so a server cannot support any other value) | Send `SetupConnection` with `protocol=3`; assert `SetupConnection.Error`. Needs a hand-built frame, since a typed client cannot encode an undefined `protocol` | `SetupConnection.Error` |
| C-SC-11 | Ω1 | Θ1 | Server accepts `SetupConnection` with `REQUIRES_STANDARD_JOBS` but sets `REQUIRES_EXTENDED_CHANNELS` in `.Success.flags` (an unsatisfiable feature set) | [§5.3.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#531-setupconnection-flags-for-mining-protocol) "`REQUIRES_EXTENDED_CHANNELS` cannot be honored for a client that set `REQUIRES_STANDARD_JOBS` in `SetupConnection.flags`" | Send `SetupConnection` with `REQUIRES_STANDARD_JOBS`; if `.Success`, assert `REQUIRES_EXTENDED_CHANNELS` is unset; if `.Error`, C-SCE-3 covers the flags | `SetupConnection.Success`, `SetupConnection.Error` |
| C-SC-12 | Ω1 | Θ1 | Server does not treat a non-printable byte in a `SetupConnection` `STR0_255` field (`endpoint_host`, `vendor`, `hardware_version`, `firmware`, `device_id`) as a protocol error (accepts the message or silently ignores the bad field) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `STR0_255` "Every byte MUST be a printable ASCII character (0x20 to 0x7E). A receiver MUST treat any other byte as a protocol error." | Send a byte-level `SetupConnection` with one `STR0_255` field at a time set to a control byte or a byte > 0x7E (all other fields valid); assert `SetupConnection.Error` or disconnect | `SetupConnection.Success`, `SetupConnection.Error` |

#### `SetupConnection.Success` ([§3.6.2](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#362-setupconnectionsuccess-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| C-SCS-1 | Ω1 | Θ1 | Server fails to respond to `SetupConnection` with a properly framed `.Success` or `.Error` — silence, wrong message type, or TCP close instead of a protocol error | [§3.6.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#361-setupconnection-client---server) "Server MUST respond with either a `SetupConnection.Success` or `SetupConnection.Error` message" | Assert framed `.Success` or `.Error` within timeout; assert not bare TCP close | `SetupConnection.Error` |
| C-SCS-2 | Ω1 | Θ1 | `used_version` outside client's `[min_version, max_version]` | [§3.6.2](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#362-setupconnectionsuccess-server---client) `used_version` "Selected version proposed by the connecting node that the upstream node supports" | Send `min_version = max_version = 2`; assert `used_version == 2` | — |

#### `SetupConnection.Error` ([§3.6.3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#363-setupconnectionerror-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| C-SCE-1 | Ω1 | Θ1 | Client sets all flags (feature probing); server errors with non-zero flags that are missing or mismatched | [§3.6.3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#363-setupconnectionerror-server---client) "MUST provide the full set of flags which it does not support" | All-flags probe; if flags ≠ 0 assert they exactly equal the unsupported set; flags=0 means rejection for an unrelated reason | — |
| C-SCE-2 | Ω1 | Θ1 | Server reports different supported/unsupported flag sets across connections to the same hostname and port | [§3.6.3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#363-setupconnectionerror-server---client) "MUST consistently support the same set of flags across all servers on the same hostname and port number" | Repeat identical all-flags probe across multiple fresh connections; compare the returned Error flags sets | — |
| C-SCE-3 | Ω1 | Θ1 | Server rejects a `SetupConnection` carrying `REQUIRES_STANDARD_JOBS` without setting `REQUIRES_STANDARD_JOBS` in `SetupConnection.Error.flags` | [§5.3.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#531-setupconnection-flags-for-mining-protocol) "MUST instead reject that client's `SetupConnection` with `SetupConnection.Error`, with `REQUIRES_STANDARD_JOBS` set in the error's `flags` field", [§3.6.3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#363-setupconnectionerror-server---client) | Send the same `SetupConnection` with and without bit 0; if the one without succeeds and the one with fails, assert `Error.flags` has bit 0 set | `SetupConnection.Success` |

#### `ChannelEndpointChanged` ([§3.6.4](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#364-channelendpointchanged-server---client))

No scenarios yet — relevant when the target is a proxy.

#### `Reconnect` ([§3.6.5](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#365-reconnect-server---client))

No scenarios yet.

### Mining Protocol Messages

#### `OpenStandardMiningChannel` ([§5.3.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#532-openstandardminingchannel-client---server))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-OSMC-1 | Ω1 | Θ1 | Returned `target` is *above* client's `max_target` and no `OpenMiningChannel.Error` sent | [§5.3.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#532-openstandardminingchannel-client---server) "Server MUST accept the target or respond by sending OpenMiningChannel.Error" | Assert `target <= max_target` | `OpenStandardMiningChannel.Success`, `OpenMiningChannel.Error` |
| M-OSMC-2 | Ω2 | Θ1 | Client completes `SetupConnection.Success` on a Mining Protocol connection, then opens no channels — server fails to close the idle connection within a reasonable period | [§5.3.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#532-openstandardminingchannel-client---server) (server SHOULD close idle connections) | Connect, handshake, wait; assert connection timeout + disconnect | — |
| M-OSMC-3 | Ω1 | Θ1 | A valid `OpenStandardMiningChannel` receives neither `.Success` nor `.Error` | [§5.3.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#532-openstandardminingchannel-client---server) "Server MUST accept the target or respond by sending OpenMiningChannel.Error" | Send a satisfiable request; assert a terminal `.Success` or `.Error` within timeout; silence violates the exhaustive accept-or-error MUST | — |
| M-OSMC-4 | Ω1 | Θ1 | Server that did not set `REQUIRES_EXTENDED_CHANNELS` rejects a satisfiable `OpenStandardMiningChannel` | [§5.2.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#522-extended-channel) "MUST also accept Standard Channels unless they set `REQUIRES_EXTENDED_CHANNELS`" | After a `.Success` without `REQUIRES_EXTENDED_CHANNELS`, send a satisfiable `OpenStandardMiningChannel`; assert `.Success` | `SetupConnection.Success`, `OpenStandardMiningChannel.Success`, `OpenMiningChannel.Error` |
| M-OSMC-5 | Ω1 | Θ1 | Server does not treat a non-printable byte in `OpenStandardMiningChannel.user_identity` as a protocol error (accepts the channel open) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `STR0_255` "Every byte MUST be a printable ASCII character (0x20 to 0x7E). A receiver MUST treat any other byte as a protocol error.", [§5.3.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#532-openstandardminingchannel-client---server) `user_identity` is `STR0_255` | Send a byte-level `OpenStandardMiningChannel` with `user_identity` containing a control byte or a byte > 0x7E; assert `OpenMiningChannel.Error` or disconnect | `OpenStandardMiningChannel.Success`, `OpenMiningChannel.Error` |
| M-OSMC-6 | Ω1 | Θ1 | Server does not treat a non-finite or negative `nominal_hash_rate` in `OpenStandardMiningChannel` as a protocol error (accepts the channel open) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `F32` "A receiver MUST treat NaN, ±Infinity, and negative values (including -0.0) as protocol errors.", [§5.3.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#532-openstandardminingchannel-client---server) `nominal_hash_rate` is `F32` | Send `OpenStandardMiningChannel` with each invalid `F32` bit pattern (`NaN`, `+Infinity`, `-Infinity`, `-0.0`, negative); assert `OpenMiningChannel.Error` or disconnect | `OpenStandardMiningChannel.Success`, `OpenMiningChannel.Error` |

#### `OpenStandardMiningChannel.Success` ([§5.3.3](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#533-openstandardminingchannelsuccess-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-OSMCS-1 | Ω1 | Θ1 | `.Success.request_id` doesn't echo the request | [§5.3.3](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#533-openstandardminingchannelsuccess-server---client) `request_id` "Client-specified request ID from OpenStandardMiningChannel message, so that the client can pair responses with open channel requests" | Random `request_id`s; assert pairing | — |
| M-OSMCS-2 | Ω1 | Θ1 | `channel_id` reused for a second channel | [§5.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#52-channel) "There MUST NOT be two Channels with the same ID" | Open N channels; assert uniqueness | — |
| M-OSMCS-3 | Ω1 | Θ1 | Same `extranonce_prefix` assigned to two standard channels → overlapping search space, duplicate work | [§5.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#51-job) "Mining servers MUST assign a unique subset of the search space to each mining device" | Open 2+ channels; compare prefixes | — |

#### `OpenExtendedMiningChannel` ([§5.3.4](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#534-openextendedminingchannel-client---server))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-OEMC-1 | Ω1 | Θ1 | Server doesn't support Extended channels at all | [§5.2.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#522-extended-channel) "Upstream servers which accept connections and provide work MUST support Extended Channels" | `OpenExtendedMiningChannel` must not error | `OpenMiningChannel.Error` |
| M-OEMC-2 | Ω1 | Θ1 | A valid `OpenExtendedMiningChannel` receives neither `.Success` nor `.Error` | [§5.3.4](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#534-openextendedminingchannel-client---server) "Server MUST satisfy the `min_extranonce_size` or respond with `OpenMiningChannel.Error`" | Send a satisfiable request; assert a terminal `.Success` or `.Error` within timeout; silence violates the exhaustive accept-or-error MUST | — |
| M-OEMC-3 | Ω1 | Θ1 | Server does not treat a non-printable byte in `OpenExtendedMiningChannel.user_identity` as a protocol error (accepts the channel open) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `STR0_255` "Every byte MUST be a printable ASCII character (0x20 to 0x7E). A receiver MUST treat any other byte as a protocol error.", [§5.3.4](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#534-openextendedminingchannel-client---server) inherits `user_identity` as `STR0_255` from `OpenStandardMiningChannel` | Send a byte-level `OpenExtendedMiningChannel` with `user_identity` containing a control byte or a byte > 0x7E; assert `OpenMiningChannel.Error` or disconnect | `OpenExtendedMiningChannel.Success`, `OpenMiningChannel.Error` |
| M-OEMC-4 | Ω1 | Θ1 | Server does not treat a non-finite or negative `nominal_hash_rate` in `OpenExtendedMiningChannel` as a protocol error (accepts the channel open) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `F32` "A receiver MUST treat NaN, ±Infinity, and negative values (including -0.0) as protocol errors.", [§5.3.4](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#534-openextendedminingchannel-client---server) inherits `nominal_hash_rate` as `F32` from `OpenStandardMiningChannel` | Send `OpenExtendedMiningChannel` with each invalid `F32` bit pattern (`NaN`, `+Infinity`, `-Infinity`, `-0.0`, negative); assert `OpenMiningChannel.Error` or disconnect | `OpenExtendedMiningChannel.Success`, `OpenMiningChannel.Error` |

#### `OpenExtendedMiningChannel.Success` ([§5.3.5](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#535-openextendedminingchannelsuccess-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-OEMCS-1 | Ω1 | Θ1 | `.Success.request_id` doesn't echo the request | [§5.3.5](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#535-openextendedminingchannelsuccess-server---client) `request_id` "Client-specified request ID from OpenExtendedMiningChannel message, so that the client can pair responses with open channel requests" | Random `request_id`s; assert pairing | — |
| M-OEMCS-2 | Ω1 | Θ1 | `channel_id` reused for a second channel | [§5.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#52-channel) "There MUST NOT be two Channels with the same ID" | Open N channels; assert uniqueness | — |
| M-OEMCS-3 | Ω1 | Θ1 | Extended channel: `extranonce_size` smaller than requested `min_extranonce_size`, without error | [§5.3.4](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#534-openextendedminingchannel-client---server) "Server MUST satisfy the `min_extranonce_size` or respond with `OpenMiningChannel.Error`" / [§5.3.5](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#535-openextendedminingchannelsuccess-server---client) | Assert `extranonce_size >= min_extranonce_size` (or explicit error) | `OpenMiningChannel.Error` |
| M-OEMCS-4 | Ω1 | Θ1 | Channels in the same group have different full extranonce sizes | [§5.2.3](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#523-group-channel) "All channels under the same Group Channel (Extended and Standard) MUST have the exact same size for the full Extended Extranonce", [§5.1.2.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5121-extended-extranonce) | Cross-check full extranonce sizes within group (only if the server uses groups) | `SetGroupChannel`, `NewExtendedMiningJob` |
| M-OEMCS-5 | Ω1 | Θ1 | Same extranonce space assigned to two extended channels → overlapping search space, duplicate work | [§5.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#51-job) "Mining servers MUST assign a unique subset of the search space to each mining device" | Open 2+ channels; compare prefixes | — |
| M-OEMCS-6 | Ω1 | Θ1 | Server negotiates `extranonce_size > 32`, making a valid `SubmitSharesExtended` impossible because `extranonce` is `B0_32` | [§5.3.4](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#534-openextendedminingchannel-client---server) / [§5.3.5](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#535-openextendedminingchannelsuccess-server---client) "extranonce_size MUST be at most 32 bytes" | Request `min_extranonce_size = 33`; assert `OpenMiningChannel.Error`. For successful opens, assert `extranonce_size <= 32` | `OpenMiningChannel.Error` |
| M-OEMCS-7 | Ω1 | Θ1 | Extended channel: returned `target` is *above* the requested `max_target` and no `OpenMiningChannel.Error` sent | [§5.3.5](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#535-openextendedminingchannelsuccess-server---client) "MUST NOT exceed `OpenExtendedMiningChannel.max_target`; a server that cannot satisfy this MUST respond with `OpenMiningChannel.Error` instead" | Assert `target <= max_target` | `OpenExtendedMiningChannel`, `OpenMiningChannel.Error` |

#### `OpenMiningChannel.Error` ([§5.3.6](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#536-openminingchannelerror-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-OMCE-1 | Ω1 | Θ1 | `request_id` not echoed on error, or error referencing a `request_id` never sent → response cannot be paired with a request | [§5.3.6](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#536-openminingchannelerror-server---client) `request_id` "Client-specified request ID from OpenMiningChannel message" | Force an open error; assert pairing; assert no unsolicited errors | — |

#### `UpdateChannel` ([§5.3.7](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#537-updatechannel-client---server))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-UC-1 | Ω1 | Θ1 | `UpdateChannel` with a `max_target` *smaller than the channel's current target* is ignored (no `SetTarget` within the new bound follows) | [§5.3.7](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#537-updatechannel-client---server) "When `max_target` is smaller than the channel’s current target, the upstream node MUST reflect the client’s request (and send an appropriate `SetTarget` message)" | Send `UpdateChannel` with `max_target` below the current target; await a `SetTarget` with `target <= max_target` (or `UpdateChannel.Error`) within the grace period; see M-ST-1 for crossing | `SetTarget`, `UpdateChannel.Error` |
| M-UC-2 | Ω1 | Θ1 | Server does not treat a non-finite or negative `nominal_hash_rate` in `UpdateChannel` as a protocol error (accepts the update or silently ignores the value) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `F32` "A receiver MUST treat NaN, ±Infinity, and negative values (including -0.0) as protocol errors.", [§5.3.7](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#537-updatechannel-client---server) `nominal_hash_rate` is `F32` | After opening a channel, send `UpdateChannel` with each invalid `F32` bit pattern (`NaN`, `+Infinity`, `-Infinity`, `-0.0`, negative); assert `UpdateChannel.Error` or disconnect | `UpdateChannel.Error` |

#### `UpdateChannel.Error` ([§5.3.8](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#538-updatechannelerror-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-UCE-1 | Ω2 | Θ1 | `UpdateChannel` for a never-opened or closed `channel_id` gets no `UpdateChannel.Error` (crash, silence, or acceptance instead) | [§5.3.8](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#538-updatechannelerror-server---client) "Sent only when `UpdateChannel` message is invalid" (derived) | Send `UpdateChannel` with a never-opened and with a closed `channel_id`; assert `UpdateChannel.Error` for each | — |
| M-UCE-2 | Ω1 | Θ1 | `UpdateChannel.Error` sent for a valid `UpdateChannel` (false rejection) | [§5.3.8](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#538-updatechannelerror-server---client) "Sent only when `UpdateChannel` message is invalid. When it is accepted by the server, no response is sent back" | Send valid `UpdateChannel`; assert no error response | — |

#### `CloseChannel` ([§5.3.9](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#539-closechannel-client---server-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-CC-1 | Ω1 | Θ1 | Server sends any message for a channel after the client's `CloseChannel` (jobs, `SetTarget`, or anything else channel-scoped) | [§5.3.9](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#539-closechannel-client---server-server---client) "The server MUST stop sending messages for the channel" | Close channel; after a short grace period for messages already in flight, assert no further message for that `channel_id` for the rest of the session | `NewMiningJob`, `NewExtendedMiningJob`, `SetTarget` |
| M-CC-2 | Ω3 | Θ1 | Server-initiated `CloseChannel` mid-session without `reason_code`; client cannot re-open cleanly | [§5.3.9](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#539-closechannel-client---server-server---client) | Resilience test (only if the server closes a channel on its own) | — |
| M-CC-3 | Ω1 | Θ1 | `CloseChannel` addressing a group channel doesn't close member channels (or closes unrelated ones) | [§5.3.9](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#539-closechannel-client---server-server---client) "all channels belonging to such group MUST be closed" | Group topology probe (only if the server uses groups) | `SetGroupChannel` |
| M-CC-4 | Ω1 | Θ1 | `CloseChannel` `reason_code` contains bytes outside printable ASCII (control characters or non-printable bytes) | [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) `STR0_255` "Every byte MUST be a printable ASCII character (0x20 to 0x7E). A receiver MUST treat any other byte as a protocol error." ([§5.3.9](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#539-closechannel-client---server-server---client) defines `reason_code` as `STR0_255`) | Validate `reason_code` string; assert printable ASCII only (only if the server closes a channel on its own) | — |
| M-CC-5 | Ω1 | Θ2 | Closing one channel (individual, not group) accidentally closes or rejects valid work on another channel on the same connection | [§5.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#52-channel) "All Channels are independent of each other" | Open channels A and B, close A, submit a valid share on B; assert `SubmitShares.Success` | `SubmitSharesStandard`, `SubmitSharesExtended`, `SubmitShares.Success` |

#### `SetExtranoncePrefix` ([§5.3.10](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5310-setextranonceprefix-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-SEP-1 | Ω1 | Θ2 | `SetExtranoncePrefix` applied retroactively to jobs sent before the change (validation mismatch) | [§5.3.10](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5310-setextranonceprefix-server---client) "applicable for all jobs sent after this message" | Grind a valid share on a pre-change job with the old prefix; assert acceptance (only if the server sends `SetExtranoncePrefix`) | `NewExtendedMiningJob`, `SubmitSharesExtended`, `SubmitShares.Success` |
| M-SEP-2 | Ω1 | Θ1 | `SetExtranoncePrefix` addressed to a group channel; clients MUST ignore it, so a prefix change the server believes it applied to the group's members never takes effect and share validation drifts | [§5.3.10](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5310-setextranonceprefix-server---client) "This message is applicable only for explicitly opened extended channels or standard channels (not group channels)" | Assert no `SetExtranoncePrefix` carries a `group_channel_id` (only if the server uses groups and sends `SetExtranoncePrefix`) | `SetGroupChannel` |

#### `SubmitSharesStandard` ([§5.3.11](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5311-submitsharesstandard-client---server))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-SSS-1 | Ω1 | Θ2 | Valid share (meets channel target, correct job/ntime/extranonce) rejected — e.g. endianness bugs in U256 target comparison | [§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client) `target` "Maximum value of produced hash that will be accepted by a server to accept shares" | Grind real share; assert `SubmitShares.Success` | `SubmitShares.Success` |
| M-SSS-2 | Ω1 | Θ2 | Invalid share accepted (hash above target) — breaks difficulty accounting; Mining Protocol Server reports hashrate that doesn't exist | [§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client) "All submits leading to hashes higher than the specified target will be rejected" | Submit share just above target; assert `SubmitShares.Error` | `SubmitShares.Error` |
| M-SSS-3 | Ω2 | Θ2 | Share accepted with `ntime` below the `ntime_start` of the referenced job | [§5.3.11](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5311-submitsharesstandard-client---server) "MUST be greater than or equal to the `ntime_start` of the referenced job" (derived: the MUST is on the client) | Grind an otherwise-valid share (meets the channel target) with `ntime < ntime_start`; expect `SubmitShares.Error` (no protocol-level upper bound — tolerance is server-local policy) | `SetNewPrevHash`, `SubmitShares.Error` |
| M-SSS-4 | Ω3 | Θ2 | Duplicate share counted twice — no dedup on `(job_id, nonce, ntime, version)` | Expected Mining Protocol Server behavior (spec-permissive — not a spec compliance requirement) | Resubmit identical valid share; assert error or unchanged accounting | `SubmitShares.Success` |
| M-SSS-5 | Ω2 | Θ2 | `SubmitSharesStandard` for a never-opened `channel_id` is accepted (expected: `SubmitShares.Error`) | [§5.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#52-channel) channel scoping (derived) | Grind an otherwise-valid share (meets the channel target) on the open channel's current job; send it with a never-opened `channel_id`; assert `SubmitShares.Error` | `SubmitShares.Error` |
| M-SSS-6 | Ω2 | Θ2 | `SubmitSharesStandard` on an extended channel is accepted (expected: `SubmitShares.Error`, since the message cannot carry the extranonce the channel's coinbase needs) | [§5.3.12](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5312-submitsharesextended-client---server) "Only relevant for extended channels" (derived: nothing states `SubmitSharesStandard` is invalid on an extended channel) | Grind a share meeting the channel target over the coinbase assembled without an `extranonce` (`coinbase_tx_prefix` + `extranonce_prefix` + `coinbase_tx_suffix`); send it as `SubmitSharesStandard` on the open extended channel; assert `SubmitShares.Error` | `SubmitSharesExtended`, `SubmitShares.Error` |
| M-SSS-7 | Ω1 | Θ2 | Server sets `REQUIRES_FIXED_VERSION` in `SetupConnection.Success.flags` but accepts a Standard share whose BIP323 general-purpose version bits differ from the job | [§5.3.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#531-setupconnection-flags-for-mining-protocol) "Upstream node will not accept any changes to the version field" | Grind a valid share after modifying only the BIP323 general-purpose version bits; assert `SubmitShares.Error` (only if the server sets `REQUIRES_FIXED_VERSION`) | `SubmitShares.Error` |
| M-SSS-8 | Ω2 | Θ2 | `SubmitSharesStandard` on a live channel referencing a `job_id` never sent on it is accepted (expected: `SubmitShares.Error`) | [§5.3.11](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5311-submitsharesstandard-client---server) `job_id` "Identifier of the job as provided by NewMiningJob or NewExtendedMiningJob message" (derived) | Grind an otherwise-valid share (meets the channel target) on the open channel's current job; send it with a `job_id` never sent on the channel; assert `SubmitShares.Error` | `SubmitShares.Error` |

#### `SubmitSharesExtended` ([§5.3.12](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5312-submitsharesextended-client---server))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-SSE-1 | Ω1 | Θ2 | Valid share rejected (extended channel) | [§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client) `target` "Maximum value of produced hash that will be accepted by a server to accept shares" | Grind real share; assert `SubmitShares.Success` | `SubmitShares.Success` |
| M-SSE-2 | Ω1 | Θ2 | Invalid share accepted (extended channel) | [§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client) "All submits leading to hashes higher than the specified target will be rejected" | Submit share just above target; assert `SubmitShares.Error` | `SubmitShares.Error` |
| M-SSE-3 | Ω2 | Θ2 | Wrong `extranonce` length accepted on an extended channel | [§5.3.12](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5312-submitsharesextended-client---server) "MUST be equal to the negotiated extranonce size" (derived: the MUST is on the client) | Grind an otherwise-valid share (meets the channel target) over a coinbase assembled with an off-by-one extranonce length; assert `SubmitShares.Error` | `SubmitShares.Error` |
| M-SSE-4 | Ω2 | Θ2 | Share accepted with `ntime` below the `ntime_start` of the referenced job (extended channel) | [§5.3.11](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5311-submitsharesstandard-client---server) "MUST be greater than or equal to the `ntime_start` of the referenced job" (derived: the MUST is on the client) | Grind an otherwise-valid share (meets the channel target) with `ntime < ntime_start`; expect `SubmitShares.Error` (no protocol-level upper bound — tolerance is server-local policy) | `SetNewPrevHash`, `SubmitShares.Error` |
| M-SSE-5 | Ω3 | Θ2 | Duplicate share counted twice (extended channel) | Expected Mining Protocol Server behavior (spec-permissive — not a spec compliance requirement) | Resubmit identical valid share; assert error or unchanged accounting | `SubmitShares.Success` |
| M-SSE-6 | Ω2 | Θ2 | `SubmitSharesExtended` for a never-opened `channel_id` is accepted (expected: `SubmitShares.Error`) | [§5.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#52-channel) channel scoping (derived) | Grind an otherwise-valid share (meets the channel target) on the open channel's current job; send it with a never-opened `channel_id`; assert `SubmitShares.Error` | `SubmitShares.Error` |
| M-SSE-7 | Ω1 | Θ2 | `SubmitSharesExtended` on a standard channel is accepted (expected: `SubmitShares.Error`) | [§5.3.12](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5312-submitsharesextended-client---server) "Only relevant for extended channels" | Grind a share valid for the standard channel's job (meets the channel target); send it as `SubmitSharesExtended` with arbitrary `extranonce` bytes; assert `SubmitShares.Error` | `SubmitSharesStandard`, `SubmitShares.Error` |
| M-SSE-8 | Ω2 | Θ2 | `SubmitSharesExtended` on a live channel referencing a `job_id` never sent on it is accepted (expected: `SubmitShares.Error`) | [§5.3.11](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5311-submitsharesstandard-client---server) `job_id` "Identifier of the job as provided by NewMiningJob or NewExtendedMiningJob message" (derived) | Grind an otherwise-valid share (meets the channel target) on the open channel's current job; send it with a `job_id` never sent on the channel; assert `SubmitShares.Error` | `SubmitShares.Error` |

#### `SubmitShares.Success` ([§5.3.13](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5313-submitsharessuccess-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-SSOK-1 | Ω1 | Θ2 | Batch semantics broken: `last_sequence_number` ≠ client's last accepted seq; `new_submits_accepted_count` ≠ batch size; `new_shares_sum` ≠ sum of acknowledged shares' job difficulties; counters not reset between batches | [§5.3.13](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5313-submitsharessuccess-server---client) "their respective counters MUST be reset when a new batch starts being processed" | Submit known-difficulty shares in controlled batches; verify all three fields arithmetically | `SubmitSharesStandard`, `SubmitSharesExtended` |
| M-SSOK-2 | Ω1 | Θ2 | `.Success` acknowledging shares never submitted (`last_sequence_number` ahead of what the client sent) | [§5.3.13](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5313-submitsharessuccess-server---client) `last_sequence_number` "Most recent sequence number with a correct result" | Sequence audit | — |

#### `SubmitShares.Error` ([§5.3.14](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5314-submitshareserror-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-SSERR-1 | Ω1 | Θ1 | Errors silently dropped: incorrect submit never gets a `SubmitShares.Error` | [§5.3.14](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5314-submitshareserror-server---client) "An error is immediately submitted for every incorrect submit attempt" | Timeout assertion on every bad share | — |
| M-SSERR-2 | Ω1 | Θ1 | `SubmitShares.Error` referencing the wrong `sequence_number` (cannot be paired with the offending submit) | [§5.3.14](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5314-submitshareserror-server---client) `sequence_number` "Submission sequence number for which this error is returned" | Pairing assertion | — |

#### `NewMiningJob` ([§5.3.15](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5315-newminingjob-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-NMJ-1 | Ω1 | Θ1 | Server sends nothing (or another message) after standard channel open — the first message MUST be a `NewMiningJob` | [§5.3.15](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5315-newminingjob-server---client) "This MUST be the first message after the channel has been successfully opened" | Message-order assertion (only if the server opens a standard channel) | `OpenStandardMiningChannel.Success`, `SetNewPrevHash` |
| M-NMJ-2 | Ω1 | Θ1 | First job arrives with `ntime_start` **set** (active job before any `SetNewPrevHash` exists) | [§5.3.15](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5315-newminingjob-server---client) "This first message MUST have `ntime_start` unset (a future job)" | Assert first job is a future job (only if the server opens a standard channel) | `SetNewPrevHash` |
| M-NMJ-3 | Ω1 | Θ1 | `NewMiningJob.channel_id` references an extended/group channel | [§5.3.15](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5315-newminingjob-server---client) "this must be a standard channel" | Type/channel-kind cross-check | — |
| M-NMJ-4 | Ω1 | Θ1 | `job_id` collisions across simultaneously valid jobs | [§5.3.15](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5315-newminingjob-server---client) "The server MUST NOT assign a `job_id` that is already in use by another currently valid job on the same channel" | Track `job_id` space | `SetNewPrevHash` |
| M-NMJ-5 | Ω1 | Θ2 | Mining Protocol Server rejects shares with rolled BIP323 general-purpose version bits despite advertising version rolling | [§5.3.15](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5315-newminingjob-server---client) `version` "The general purpose bits (as specified in BIP323) can be freely manipulated by the downstream node" | Submit valid share with rolled version bits; assert acceptance when rolling allowed | `SubmitSharesStandard`, `SubmitShares.Error`, `SubmitShares.Success` |
| M-NMJ-6 | Ω1 | Θ1 | Active job arrives with `ntime_start` lower than the `ntime_start` of the most recent `SetNewPrevHash` (job mined against that `prev_hash`) | [§5.3.15](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5315-newminingjob-server---client) "When set, `ntime_start` MUST NOT be lower than the `ntime_start` of the most recent `SetNewPrevHash` applicable to the channel" | After `SetNewPrevHash` activates a job, assert every subsequent active job's `ntime_start >= SetNewPrevHash.ntime_start` | `SetNewPrevHash` |

#### `NewExtendedMiningJob` ([§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-NEMJ-1 | Ω1 | Θ1 | Server sends nothing (or another message) after extended channel open — the first message MUST be a `NewExtendedMiningJob` | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client) "This MUST be the first message after an extended channel has been successfully opened" | Message-order assertion | `OpenExtendedMiningChannel.Success`, `SetNewPrevHash` |
| M-NEMJ-2 | Ω1 | Θ1 | First extended job arrives with `ntime_start` **set** (active job before any `SetNewPrevHash` exists) | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client) "This first message MUST have `ntime_start` unset" | Assert first job is a future job | `SetNewPrevHash` |
| M-NEMJ-3 | Ω1 | Θ1 | `NewExtendedMiningJob` targets a standard channel | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client) "Extended and group channels only" | Type/channel-kind cross-check | — |
| M-NEMJ-4 | Ω1 | Θ1 | `NewExtendedMiningJob` sent on a connection where client set `REQUIRES_STANDARD_JOBS` | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client), [§5.2.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#521-standard-channel) "restricted to Standard Jobs (via `REQUIRES_STANDARD_JOBS` bit flag of `SetupConnection` message)" | Assert absence (only if the server accepts `REQUIRES_STANDARD_JOBS`) | `SetupConnection` |
| M-NEMJ-5 | Ω1 | Θ1 | BIP141 stripping violation: `coinbase_tx_prefix`/`coinbase_tx_suffix` still contain marker/flag/witness data → client-computed coinbase txid and merkle root are wrong → every share built on the job is garbage | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client) "MUST be stripped of BIP141 fields" | Reconstruct coinbase on extended channel; assert no witness fields present | — |
| M-NEMJ-6 | Ω1 | Θ1 | `job_id` collisions across simultaneously valid jobs (extended) | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client) "The server MUST NOT assign a `job_id` that is already in use by another currently valid job on the same channel" | Track `job_id` space | `SetNewPrevHash` |
| M-NEMJ-7 | Ω1 | Θ2 | Mining Protocol Server rejects shares with rolled BIP323 bits despite `version_rolling_allowed = True` | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client) `version_rolling_allowed` "If set to True, the general purpose bits of version (as specified in BIP323) can be freely manipulated by the downstream node" | Submit valid share with rolled version bits; assert acceptance when rolling allowed | `SubmitSharesExtended`, `SubmitShares.Error`, `SubmitShares.Success` |
| M-NEMJ-8 | Ω1 | Θ1 | Client sets `REQUIRES_VERSION_ROLLING` in `SetupConnection.flags`; server accepts but later sends `NewExtendedMiningJob` with `version_rolling_allowed = False` | [§5.3.1](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#531-setupconnection-flags-for-mining-protocol) "server MUST NOT send jobs which do not allow version rolling" | Assert `version_rolling_allowed` is always True when `REQUIRES_VERSION_ROLLING` was set | `NewExtendedMiningJob` |
| M-NEMJ-9 | Ω1 | Θ2 | Server sends `NewExtendedMiningJob` with `version_rolling_allowed = False` but accepts a share whose BIP323 general-purpose version bits differ from the job | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client) "If set to False, the downstream node MUST use version as it is defined by this message, and the upstream node MUST reject shares for this job whose `version` differs from it" | Grind a valid share after modifying only the BIP323 general-purpose version bits; assert `SubmitShares.Error` (only if the server sends a job with `version_rolling_allowed = False`) | `SubmitSharesExtended`, `SubmitShares.Error` |
| M-NEMJ-10 | Ω1 | Θ1 | Active extended job arrives with `ntime_start` lower than the `ntime_start` of the most recent `SetNewPrevHash` (job mined against that `prev_hash`) | [§5.3.16](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5316-newextendedminingjob-server---client) "When set, `ntime_start` MUST NOT be lower than the `ntime_start` of the most recent `SetNewPrevHash` applicable to the channel" | After `SetNewPrevHash` activates a job, assert every subsequent active job's `ntime_start >= SetNewPrevHash.ntime_start` | `SetNewPrevHash` |

#### `SetNewPrevHash` ([§5.3.17](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5317-setnewprevhash-server---client-broadcast))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-SNPH-1 | Ω1 | Θ1 | `SetNewPrevHash` activates a `job_id` never sent as a future job | [§5.3.17](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5317-setnewprevhash-server---client-broadcast) "The server MUST NOT send this message referencing a `job_id` that was not previously sent as a future job (i.e. with `ntime_start` unset) on the corresponding channel" | Cross-reference announced future jobs | `NewMiningJob`, `NewExtendedMiningJob` |
| M-SNPH-2 | Ω2 | Θ2 | Mining Protocol Server keeps accepting shares for jobs invalidated by a `SetNewPrevHash` (stale acceptance → wrong accounting, misleading hashrate) | [§5.3.17](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5317-setnewprevhash-server---client-broadcast) "Shares submitted for jobs invalidated by a previous `SetNewPrevHash` (stale shares) SHOULD be rejected by the server" | After a new block triggers a later `SetNewPrevHash`, submit an otherwise-valid share (meets the channel target) for a job it invalidated; expect `SubmitShares.Error` (only if a new block arrives during the run) | `SubmitShares.Error`, `SubmitShares.Success` |
| M-SNPH-3 | Ω1 | Θ1 | `ntime_start` in `SetNewPrevHash` not consensus-valid for the block following `prev_hash` (≤ chain median-time-past, or beyond `MAX_FUTURE_BLOCK_TIME`) → any block found would be invalid | [§5.3.17](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5317-setnewprevhash-server---client-broadcast) "The server MUST ensure that `ntime_start` is a consensus-valid nTime for the block following `prev_hash` (in particular, greater than that chain's median-time-past)" | Sanity-bound assertion in both directions | — |

#### `SetCustomMiningJob`, `SetCustomMiningJob.Success`, `SetCustomMiningJob.Error` ([§5.3.18](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5318-setcustomminingjob-client---server), [§5.3.19](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5319-setcustomminingjobsuccess-server---client), [§5.3.20](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5320-setcustomminingjoberror-server---client))

Deferred until a Job Declarator Server suite exists: exercising these messages requires the test client to play the Job Declarator Client against the target's Job Declarator Server. No scenarios yet.

#### `SetTarget` ([§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-ST-1 | Ω1 | Θ1 | `SetTarget.target` above the `max_target` most recently accepted for the channel (from channel open, or the last accepted `UpdateChannel`) | [§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client) "MUST NOT exceed the `max_target` of the most recent `UpdateChannel` the server accepted for the channel or, if there has been none, of the `OpenStandardMiningChannel` or `OpenExtendedMiningChannel` that opened it" | Assert bound on every `SetTarget`. After lowering `max_target` via `UpdateChannel`: a target above the previous `max_target` is a violation at any time; a target above the new `max_target` is a violation only once the grace period has expired without an `UpdateChannel.Error` | `OpenStandardMiningChannel`, `OpenExtendedMiningChannel`, `UpdateChannel`, `UpdateChannel.Error` |
| M-ST-2 | Ω1 | Θ2 | Mining Protocol Server applies new target retroactively to active jobs that had `ntime_start` set | [§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client) "not applicable for already received jobs with `ntime_start=nTime`" | Grind a share valid under the old target on a pre-`SetTarget` job; assert old target applies | `NewMiningJob`, `SubmitSharesStandard` |
| M-ST-3 | Ω1 | Θ2 | A `SetTarget` update for one channel (individual, not group) changes validation on another independent channel | [§5.2](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#52-channel) "All Channels are independent of each other" | Open channels A and B, reduce A's target via `UpdateChannel`; grind and submit a share on B valid under B's unchanged target; assert `SubmitShares.Success` | `UpdateChannel`, `SubmitSharesStandard`, `SubmitSharesExtended`, `SubmitShares.Success` |
| M-ST-4 | Ω1 | Θ1 | `SetTarget` addressed to a group channel carries a `target` above the most recently accepted `max_target` of at least one member channel | [§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client) "When `SetTarget` is sent to a group channel, the target is applicable to all channels in the group, and the bound above applies to each of them" | Open channels with different `max_target`s that the server groups; on every group-addressed `SetTarget` assert the M-ST-1 bound against each member (only if the server uses groups) | `SetGroupChannel`, `OpenStandardMiningChannel`, `OpenExtendedMiningChannel`, `UpdateChannel` |
| M-ST-5 | Ω1 | Θ2 | `SetTarget` is not applied to a future job already received with an empty `ntime_start`; after activation the old target still governs | [§5.3.21](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5321-settarget-server---client) "The target is valid until the next `SetTarget` message is sent and is applicable for all jobs received on the channel in the future or already received with an empty `ntime_start`." | Receive a future job, provoke a `SetTarget` via `UpdateChannel`, activate that job with a `SetNewPrevHash` referencing its `job_id`, then submit a share that discriminates between the old and new targets; assert the new target governs (only if `SetTarget` and `SetNewPrevHash` are observed) | `NewMiningJob`, `NewExtendedMiningJob`, `SetNewPrevHash`, `SetTarget`, `UpdateChannel`, `SubmitSharesStandard`, `SubmitSharesExtended`, `SubmitShares.Error` |

#### `SetGroupChannel` ([§5.3.22](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5322-setgroupchannel-server---client))

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| M-SGC-1 | Ω1 | Θ1 | `SetGroupChannel` sent on a connection with `REQUIRES_STANDARD_JOBS` | [§5.3.22](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5322-setgroupchannel-server---client) "can be sent only to connections that don't have REQUIRES_STANDARD_JOBS" | Assert absence (only if the server accepts `REQUIRES_STANDARD_JOBS`) | `SetupConnection` |
| M-SGC-2 | Ω1 | Θ1 | `group_channel_id` collides with an existing mining `channel_id` | [§5.2.3](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#523-group-channel), [§5.3.22](https://github.com/stratum-mining/sv2-spec/blob/main/05-Mining-Protocol.md#5322-setgroupchannel-server---client) "MUST ensure that a group channel has a unique channel ID" | Cross-check ID spaces (only if the server uses groups) | `OpenStandardMiningChannel.Success`, `OpenExtendedMiningChannel.Success` |

### Protocol Extensions ([§3.4](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#34-protocol-extensions))

One subsection per registered extension; future registrations append new subsections. Generic extension machinery (unknown `extension_type` handling, TLV invariants, negotiation-before-use) lives in the [General](#general-3135-5152-8) table as GEN-6…GEN-11.

**Applicability caveat:** a server that implements no extensions is compliant — it simply ignores `RequestExtensions`. Therefore *no response* to `RequestExtensions` is not a failure; the `X0001-*` scenarios below only apply once a server engages with the negotiation protocol.

#### Extension `0x0001` — [Extensions Negotiation](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md)

##### `RequestExtensions`

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| X0001-REQ-1 | Ω2 | Θ1 | Server requires extensions the client didn't request, but keeps the connection open after the client doesn't retry | ext `0x0001` [§4](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#4-implementation-notes) example 6 "If the client does not retry with the required extension, the server MUST disconnect the client" (derived: the MUST appears only in a worked example, not in the Error Handling notes) | Omit a server-required extension from the request; do not retry; assert disconnect (only if the server requires an extension) | `RequestExtensions.Error` |

##### `RequestExtensions.Success`

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| X0001-REQS-1 | Ω1 | Θ1 | `request_id` doesn't echo the request | ext `0x0001` [§2](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#2-messages-defined-by-this-extension) `request_id` "Unique identifier for pairing the response" | Random `request_id`s; assert pairing | — |
| X0001-REQS-2 | Ω1 | Θ1 | Extension-defined message framed with the wrong `extension_type` (must be `0x0001`, not `0x0000` or `0x8001`) | [§3.4.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#341-extension-type-field-usage) "MUST have `extension_type` set to the identifier of the extension that introduced them", ext `0x0001` [§3](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#3-message-types) | Assert raw `extension_type == 0x0001` (the extension's message table sets `channel_msg = 0`) on every negotiation frame | `RequestExtensions.Error` |
| X0001-REQS-3 | Ω2 | Θ1 | `RequestExtensions.Success.supported_extensions` contains extension IDs that were not in the client's request | ext `0x0001` [§1](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#1-overview) (derived — negotiation semantics: Success confirms which of the requested extensions are supported) | Request a known set of extensions; assert `supported_extensions` ⊆ requested set | — |

##### `RequestExtensions.Error`

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| X0001-REQE-1 | Ω1 | Θ1 | None of the requested extensions are supported, but server doesn't respond with `RequestExtensions.Error` | ext `0x0001` [§4](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#4-implementation-notes) ("MUST respond with `RequestExtensions.Error` if none of the requested extensions are supported") | Request only unallocated extension IDs; assert `Error` | — |
| X0001-REQE-2 | Ω1 | Θ1 | `Error` contents wrong: `unsupported_extensions` ≠ the requested-but-unsupported set | ext `0x0001` [§1.1](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#11-negotiating-extensions) "identifying unsupported extensions and required extensions from the responder", [§4](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#4-implementation-notes) | Force an `Error` response (request only unallocated IDs, or omit a server-required extension); assert `unsupported_extensions` equals the requested-but-unsupported set. A `Success` listing a supported subset is compliant for a partial request (ext `0x0001` §4 example 4) | — |
| X0001-REQE-3 | Ω1 | Θ1 | `request_id` doesn't echo the request | ext `0x0001` [§2](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#2-messages-defined-by-this-extension) `request_id` "Unique identifier for pairing the response" | Random `request_id`s; assert pairing | — |
| X0001-REQE-4 | Ω1 | Θ1 | `Error.required_extensions` omits an extension the server requires but the client did not request | ext `0x0001` [§4](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0001-extensions-negotiation.md#4-implementation-notes) "MUST list every omitted required extension in `required_extensions`" | Request without a server-required extension; assert `required_extensions` contains it (only if the server requires an extension) | — |

#### Extension `0x0002` — [Worker-Specific Hashrate Tracking](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0002-worker-specific-hashrate-tracking.md)

This extension defines no new messages; it extends `SubmitSharesExtended` with a `user_identity` TLV field. Generic TLV machinery lives in the [General](#general-3135-5152-8) table as GEN-7…GEN-9.

##### `SubmitSharesExtended` (TLV-extended)

| Scenario IDs | Compliance tier | Testability tier | Failure mode | Spec basis | Detection | Involves |
|---|---|---|---|---|---|---|
| X0002-SSE-1 | Ω1 | Θ2 | Server accepts a `SubmitSharesExtended` whose negotiated `0x0002` TLV carries a non-printable `user_identity` value | ext `0x0002` [§1.1](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0002-worker-specific-hashrate-tracking.md#11-tlv-format-for-user_identity) "Subject to the same byte rule as `STR0_255`", [§3.1](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#31-data-types-mapping) "Every byte MUST be a printable ASCII character ... A receiver MUST treat any other byte as a protocol error" | Negotiate `0x0002`; submit an otherwise-valid share (meets the channel target) with a within-32-byte TLV containing a control byte; assert the share is rejected (`SubmitShares.Error` or disconnect) (only if the server supports extension `0x0002`) | `RequestExtensions`, `SubmitSharesExtended`, `SubmitShares.Error` |
| X0002-SSE-2 | Ω1 | Θ2 | Server rejects or chokes on an otherwise-valid share carrying a valid `0x0002` `user_identity` TLV | ext `0x0002` [§1.3](https://github.com/stratum-mining/sv2-spec/blob/main/extensions/0x0002-worker-specific-hashrate-tracking.md#13-behavior-based-on-negotiation) "The `user_identity` field MUST be included in `SubmitSharesExtended` as a TLV with Type `0x0002`", [§3.4.3](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#343-stratum-v2-tlv-encoding-model) "A device processing `SubmitSharesExtended` MUST scan for TLV fields matching any negotiated extensions" | Negotiate `0x0002`; submit an otherwise-valid share (meets the channel target) with a printable `user_identity` TLV; assert `SubmitShares.Success` (only if the server supports extension `0x0002`) | `RequestExtensions`, `SubmitSharesExtended`, `SubmitShares.Success` |

### Job Declaration Protocol Messages ([§6](https://github.com/stratum-mining/sv2-spec/blob/main/06-Job-Declaration-Protocol.md))

Reserved for a future Job Declarator Server suite. No scenarios yet.

The current spec also includes a `DeclareMiningJob.wtxid_list` block-order MUST ([§6.4.4](https://github.com/stratum-mining/sv2-spec/blob/main/06-Job-Declaration-Protocol.md#644-declareminingjob-client---server)) and the BIP141 rules for `DeclareMiningJob` ([§3.7.2](https://github.com/stratum-mining/sv2-spec/blob/main/03-Protocol-Overview.md#372-bip141-on-declareminingjob)); add `JD-*` scenarios when this suite is implemented.

### Template Distribution Protocol Messages ([§7](https://github.com/stratum-mining/sv2-spec/blob/main/07-Template-Distribution-Protocol.md))

Reserved for a future Template Distribution Server suite. No scenarios yet.

---

## Suite Coverage

✅ = covered; ❌ = not covered. The short title next to each ID is informational; the [Scenario Catalog](#scenario-catalog) is authoritative.

| Scenario IDs | Mining Protocol Server |
|---|---|
| GEN-1 — `channel_msg` bit correctness | ❌ |
| GEN-2 — `extension_type` on core messages | ❌ |
| GEN-3 — server error codes printable ASCII only | ❌ |
| GEN-4 — `channel_id` never opened | ❌ |
| GEN-5 — job starvation (liveness) | ❌ |
| GEN-6 — unknown `extension_type` discarded | ❌ |
| GEN-7 — unexpected TLV fields ignored | ❌ |
| GEN-8 — server TLV placement/ordering | ❌ |
| GEN-9 — over-length TLV rejected | ❌ |
| GEN-10 — proxy forwards unknown-extension frames (channel_id remap only) | ❌ |
| GEN-11 — no extension messages without negotiation | ❌ |
| GEN-12 — contradictory double response | ❌ |
| GEN-15 — oversized `B0_32` field | ❌ |
| C-SC-1 — no messages before `SetupConnection` response | ❌ |
| C-SC-2 — rejects empty `device_id` | ❌ |
| C-SC-3 — `REQUIRES_FIXED_VERSION` vs `REQUIRES_VERSION_ROLLING` | ❌ |
| C-SC-4 — `REQUIRES_STANDARD_JOBS` accepted by a standard-serving server | ❌ |
| C-SC-5 — standard channel accepted despite `REQUIRES_EXTENDED_CHANNELS` | ❌ |
| C-SC-6 — `REQUIRES_FIXED_VERSION` + `version_rolling_allowed` consistency | ❌ |
| C-SC-7 — unsupported `protocol` value | ✅ |
| C-SC-8 — version range with no supported version | ✅ |
| C-SC-9 — 255-byte printable device-info strings | ❌ |
| C-SC-10 — undefined `protocol` value | ❌ |
| C-SC-11 — `REQUIRES_EXTENDED_CHANNELS` on a `REQUIRES_STANDARD_JOBS` connection | ❌ |
| C-SC-12 — non-printable `STR0_255` in `SetupConnection` | ❌ |
| C-SCS-1 — `SetupConnection` response type / TCP close vs framed error | ✅ |
| C-SCS-2 — `used_version` within negotiated range | ✅ |
| C-SCE-1 — feature-probing missing/mismatched error flags | ❌ |
| C-SCE-2 — flag-set stability across connections | ❌ |
| C-SCE-3 — `REQUIRES_STANDARD_JOBS` rejection reports bit 0 | ❌ |
| M-OSMC-1 — target vs `max_target` on channel open | ❌ |
| M-OSMC-2 — inactivity timeout on idle mining connection | ❌ |
| M-OSMC-3 — no response to standard channel open | ❌ |
| M-OSMC-4 — standard channel rejected without `REQUIRES_EXTENDED_CHANNELS` | ❌ |
| M-OSMC-5 — non-printable `user_identity` (standard) | ❌ |
| M-OSMC-6 — invalid `F32` `nominal_hash_rate` (standard) | ❌ |
| M-OSMCS-1 — `request_id` echo in channel open success | ✅ |
| M-OSMCS-2 — `channel_id` uniqueness | ❌ |
| M-OSMCS-3 — `extranonce_prefix` uniqueness | ❌ |
| M-OEMC-1 — Extended channel support | ❌ |
| M-OEMC-2 — no response to extended channel open | ❌ |
| M-OEMC-3 — non-printable `user_identity` (extended) | ❌ |
| M-OEMC-4 — invalid `F32` `nominal_hash_rate` (extended) | ❌ |
| M-OEMCS-1 — `request_id` echo in extended open success | ✅ |
| M-OEMCS-2 — `channel_id` uniqueness (extended) | ❌ |
| M-OEMCS-3 — `extranonce_size` vs `min_extranonce_size` | ✅ |
| M-OEMCS-4 — same group, different full extranonce sizes | ❌ |
| M-OEMCS-5 — extranonce space uniqueness (extended) | ❌ |
| M-OEMCS-6 — `extranonce_size > 32` | ❌ |
| M-OEMCS-7 — target vs `max_target` on extended open | ❌ |
| M-OMCE-1 — `request_id` echo on open error | ❌ |
| M-UC-1 — `UpdateChannel` target reduction honored | ❌ |
| M-UC-2 — invalid `F32` `nominal_hash_rate` in `UpdateChannel` | ❌ |
| M-UCE-1 — `UpdateChannel.Error` for invalid channel | ❌ |
| M-UCE-2 — no `UpdateChannel.Error` on valid update | ❌ |
| M-CC-1 — silence after `CloseChannel` | ❌ |
| M-CC-2 — server-initiated `CloseChannel` without `reason_code` | ❌ |
| M-CC-3 — group-channel `CloseChannel` correctness | ❌ |
| M-CC-4 — `reason_code` printable ASCII only | ❌ |
| M-CC-5 — closing one channel breaks another independent channel | ❌ |
| M-SEP-1 — `SetExtranoncePrefix` non-retroactivity | ❌ |
| M-SEP-2 — `SetExtranoncePrefix` addressed to a group channel | ❌ |
| M-SSS-1 — valid-share acceptance (standard) | ❌ |
| M-SSS-2 — invalid-share rejection (standard) | ❌ |
| M-SSS-3 — `ntime` lower-bound enforcement (standard) | ❌ |
| M-SSS-4 — duplicate-share rejection (standard) | ❌ |
| M-SSS-5 — never-opened `channel_id` in share submission (standard) | ❌ |
| M-SSS-6 — `SubmitSharesStandard` on extended channel | ❌ |
| M-SSS-7 — fixed-version share accepted (standard) | ❌ |
| M-SSS-8 — unknown `job_id` in share submission (standard) | ❌ |
| M-SSE-1 — valid-share acceptance (extended) | ❌ |
| M-SSE-2 — invalid-share rejection (extended) | ❌ |
| M-SSE-3 — extranonce length validation | ❌ |
| M-SSE-4 — `ntime` lower-bound enforcement (extended) | ❌ |
| M-SSE-5 — duplicate-share rejection (extended) | ❌ |
| M-SSE-6 — never-opened `channel_id` in share submission (extended) | ❌ |
| M-SSE-7 — `SubmitSharesExtended` on standard channel | ❌ |
| M-SSE-8 — unknown `job_id` in share submission (extended) | ❌ |
| M-SSOK-1 — `SubmitShares.Success` batch accounting | ❌ |
| M-SSOK-2 — `last_sequence_number` audit | ❌ |
| M-SSERR-1 — `SubmitShares.Error` always sent for bad submits | ❌ |
| M-SSERR-2 — `SubmitShares.Error` sequence_number pairing | ❌ |
| M-NMJ-1 — first message after standard open is a job | ✅ |
| M-NMJ-2 — first standard job `ntime_start` unset | ✅ |
| M-NMJ-3 — `NewMiningJob` channel-kind cross-check | ❌ |
| M-NMJ-4 — standard `job_id` uniqueness | ❌ |
| M-NMJ-5 — version-rolling share acceptance (standard) | ❌ |
| M-NMJ-6 — active job `ntime_start` below latest `SetNewPrevHash` | ❌ |
| M-NEMJ-1 — first message after extended open is a job | ✅ |
| M-NEMJ-2 — first extended job `ntime_start` unset | ✅ |
| M-NEMJ-3 — `NewExtendedMiningJob` channel-kind cross-check | ❌ |
| M-NEMJ-4 — `NewExtendedMiningJob` vs `REQUIRES_STANDARD_JOBS` | ❌ |
| M-NEMJ-5 — BIP141 stripping in coinbase tx data | ❌ |
| M-NEMJ-6 — extended `job_id` uniqueness | ❌ |
| M-NEMJ-7 — version-rolling share acceptance (extended) | ❌ |
| M-NEMJ-8 — version-rolling loophole (`version_rolling_allowed = False` after `REQUIRES_VERSION_ROLLING`) | ❌ |
| M-NEMJ-9 — non-rolling share accepted (extended) | ❌ |
| M-NEMJ-10 — active extended job `ntime_start` below latest `SetNewPrevHash` | ❌ |
| M-SNPH-1 — `SetNewPrevHash` references known future `job_id` | ❌ |
| M-SNPH-2 — stale-share rejection | ❌ |
| M-SNPH-3 — `ntime_start` consensus-validity bounds | ❌ |
| M-ST-1 — `SetTarget` bound vs most recently accepted `max_target` | ❌ |
| M-ST-2 — `SetTarget` non-retroactivity | ❌ |
| M-ST-3 — target update leaks to another independent channel | ❌ |
| M-ST-4 — group `SetTarget` bound vs each member's `max_target` | ❌ |
| M-ST-5 — `SetTarget` applied to already-received future job | ❌ |
| M-SGC-1 — `SetGroupChannel` vs `REQUIRES_STANDARD_JOBS` | ❌ |
| M-SGC-2 — group/channel ID namespace separation | ❌ |
| X0001-REQ-1 — disconnect when required extensions ignored | ❌ |
| X0001-REQS-1 — `request_id` echo on negotiation success | ❌ |
| X0001-REQS-2 — negotiation frames carry `extension_type = 0x0001` | ❌ |
| X0001-REQS-3 — `supported_extensions` not a subset of the request | ❌ |
| X0001-REQE-1 — `Error` when nothing requested is supported | ❌ |
| X0001-REQE-2 — `Error` unsupported set exact | ❌ |
| X0001-REQE-3 — `request_id` echo on negotiation error | ❌ |
| X0001-REQE-4 — `required_extensions` completeness | ❌ |
| X0002-SSE-1 — non-printable `user_identity` TLV rejected | ❌ |
| X0002-SSE-2 — valid `user_identity` TLV accepted | ❌ |


Note: more columns will be added as the project matures (Job Declarator Server, Template Distribution Server suites).
