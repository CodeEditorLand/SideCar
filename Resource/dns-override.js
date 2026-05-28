"use strict";

// Redirect ALL Node.js DNS resolution to the embedded Hickory server
const dns = require("dns");

const { Resolver } = dns;

const [host, portStr] = (process.env.Resolve || "127.0.0.1:5380").split(":");

const servers = [`${host}:${portStr}`];

dns.setServers(servers); // affects net.lookup global path

dns.promises.setDefaultResultOrder("ipv4first");

// Also override the node:dns/promises Resolver class default
const r = new Resolver();

r.setServers(servers);

// Patch dns.lookup so http/https native modules use Hickory too
const origLookup = dns.lookup.bind(dns);

dns.lookup = (hostname, options, cb) => {
	const callback = typeof options === "function" ? options : cb;

	const opts = typeof options === "object" ? options : {};

	// .editor.land always force-resolves via Hickory (already configured above)
	origLookup(hostname, { ...opts, verbatim: true }, callback);
};
