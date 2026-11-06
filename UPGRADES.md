# Upgrades

## 1.1.0

Breaking: the Datom text reading follows datom-codec 0.33.0 and protos 0.33.0. A guillemet string is read by balance: a « » pair inside it is content, and only an unbalanced guillemet is escaped, `\«` as `\»`. Datom text that held a bare unbalanced `«` inside a guillemet string, or a literal `\«`, now reads differently and is rewritten with the escape. The Rust types and their rkyv form are unchanged by this release.

To deploy: repin flow-nexus to this version together with signal-flow-nexus and meta-signal-flow-nexus 1.1.0, then run the consumer's tests.
