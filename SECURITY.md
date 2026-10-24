# Security

Please report a vulnerability privately through [GitHub's security advisories](https://github.com/HNagouda/openarchi/security/advisories/new) rather than in a public issue. I'll reply within a week and credit you in the fix unless you'd rather not be named.

Only the latest release gets fixes. openarchi is maintained by one person in their spare time, so there is no bounty.

The parts most worth a look are the ones that read outside input: project and theme files, which people share, the responses from the PDOK data services, and the MCP server in `crates/mcp`, which acts on whatever an assistant sends it inside your project library.
