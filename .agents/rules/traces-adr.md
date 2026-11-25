# traces-adr-human-accept
paths: docs/adr/*.md
forbid: /^status:\s*accepted\b/im
An agent must not mark an ADR accepted. Write status: proposed and wait for a human review.
