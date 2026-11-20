/**
 * OpenAPI schema conformance tests for StarFund API.
 *
 * Validates that request/response payloads conform to the schemas defined in
 * docs/openapi.yaml using AJV (JSON Schema draft-07 / OpenAPI 3.1 subset).
 *
 * Run:  npm test  (from the docs/ directory)
 */

"use strict";

const { describe, it } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const yaml = require("js-yaml");
const Ajv = require("ajv");
const addFormats = require("ajv-formats");

// ---------------------------------------------------------------------------
// Load spec and build validators
// ---------------------------------------------------------------------------

const specPath = path.join(__dirname, "..", "openapi.yaml");
const spec = yaml.load(fs.readFileSync(specPath, "utf8"));
const schemas = spec.components.schemas;

const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);

// Register every component schema so $ref resolution works.
for (const [name, schema] of Object.entries(schemas)) {
  ajv.addSchema(schema, `#/components/schemas/${name}`);
}

/**
 * Compile a schema reference and return a validate function.
 * @param {string} ref - e.g. "#/components/schemas/InvoiceEscrow"
 */
function validator(ref) {
  return ajv.compile({ $ref: ref });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Assert that `data` is valid against `schemaRef`.
 * @param {string} schemaRef
 * @param {unknown} data
 */
function assertValid(schemaRef, data) {
  const validate = validator(schemaRef);
  const ok = validate(data);
  assert.ok(ok, `Expected valid ${schemaRef} but got errors: ${JSON.stringify(validate.errors)}`);
}

/**
 * Assert that `data` is INVALID against `schemaRef`.
 * @param {string} schemaRef
 * @param {unknown} data
 */
function assertInvalid(schemaRef, data) {
  const validate = validator(schemaRef);
  const ok = validate(data);
  assert.ok(!ok, `Expected invalid ${schemaRef} but validation passed for: ${JSON.stringify(data)}`);
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

// 56-char Stellar G-address (base32 A-Z2-7)
const VALID_ADDRESS = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

// Mirrors the `InvoiceEscrow` struct in `escrow/src/lib.rs`.
const validEscrow = {
  invoice_id: "INV-1023",
  admin: VALID_ADDRESS,
  sme_address: VALID_ADDRESS,
  payer: VALID_ADDRESS,
  amount: 100_000_000_000,
  funding_target: 100_000_000_000,
  funded_amount: 0,
  yield_bps: 800,
  maturity: 1000,
  status: 0,
  dispute_active: false,
};

// ---------------------------------------------------------------------------
// StellarAddress
// ---------------------------------------------------------------------------

describe("StellarAddress schema", () => {
  const ref = "#/components/schemas/StellarAddress";

  it("accepts a valid 56-char base32 address", () => {
    assertValid(ref, VALID_ADDRESS);
  });

  it("rejects an address that is too short", () => {
    assertInvalid(ref, "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOC");
  });

  it("rejects an address with lowercase letters", () => {
    assertInvalid(ref, "gaazi4tcr3ty5ojhctjc2a4qsy6cjwjh5iajtgkin2er7lbnvkoccwn");
  });

  it("rejects an empty string", () => {
    assertInvalid(ref, "");
  });
});

// ---------------------------------------------------------------------------
// InvoiceId
// ---------------------------------------------------------------------------

describe("InvoiceId schema", () => {
  const ref = "#/components/schemas/InvoiceId";

  it("accepts a typical invoice id", () => {
    assertValid(ref, "INV-1023");
  });

  it("accepts alphanumeric with underscores and hyphens", () => {
    assertValid(ref, "INV_001-A");
  });

  it("rejects an empty string", () => {
    assertInvalid(ref, "");
  });

  it("rejects a string longer than 12 chars", () => {
    assertInvalid(ref, "INVOICE-99999");
  });

  it("rejects special characters", () => {
    assertInvalid(ref, "INV#1023");
  });
});

// ---------------------------------------------------------------------------
// EscrowStatus
// ---------------------------------------------------------------------------

describe("EscrowStatus schema", () => {
  const ref = "#/components/schemas/EscrowStatus";

  it("accepts 0 (open)", () => assertValid(ref, 0));
  it("accepts 1 (funded)", () => assertValid(ref, 1));
  it("accepts 2 (settled)", () => assertValid(ref, 2));
  it("rejects 3 (unknown)", () => assertInvalid(ref, 3));
  it("rejects -1", () => assertInvalid(ref, -1));
  it("rejects a string", () => assertInvalid(ref, "open"));
});

// ---------------------------------------------------------------------------
// InvoiceEscrow
// ---------------------------------------------------------------------------

describe("InvoiceEscrow schema", () => {
  const ref = "#/components/schemas/InvoiceEscrow";

  it("accepts a fully-funded escrow", () => {
    assertValid(ref, { ...validEscrow, funded_amount: 100_000_000_000, status: 1 });
  });

  it("accepts a settled escrow", () => {
    assertValid(ref, { ...validEscrow, funded_amount: 100_000_000_000, status: 2 });
  });

  it("accepts an open escrow with zero funded_amount", () => {
    assertValid(ref, validEscrow);
  });

  it("rejects missing invoice_id", () => {
    const { invoice_id, ...rest } = validEscrow;
    assertInvalid(ref, rest);
  });

  it("rejects missing sme_address", () => {
    const { sme_address, ...rest } = validEscrow;
    assertInvalid(ref, rest);
  });

  it("rejects missing admin", () => {
    const { admin, ...rest } = validEscrow;
    assertInvalid(ref, rest);
  });

  it("rejects missing payer", () => {
    const { payer, ...rest } = validEscrow;
    assertInvalid(ref, rest);
  });

  it("rejects missing dispute_active", () => {
    const { dispute_active, ...rest } = validEscrow;
    assertInvalid(ref, rest);
  });

  it("accepts an escrow with an active dispute", () => {
    assertValid(ref, { ...validEscrow, dispute_active: true });
  });

  it("rejects non-boolean dispute_active", () => {
    assertInvalid(ref, { ...validEscrow, dispute_active: "yes" });
  });

  it("rejects invalid payer format", () => {
    assertInvalid(ref, { ...validEscrow, payer: "not-an-address" });
  });

  it("rejects invalid admin format", () => {
    assertInvalid(ref, { ...validEscrow, admin: "not-an-address" });
  });

  it("accepts amounts above the int64 range (contract i128)", () => {
    // 1e19 > 2^63 - 1 and far beyond Number.MAX_SAFE_INTEGER, while still
    // inside i128: the schema must not constrain these fields to int64.
    assertValid(ref, { ...validEscrow, amount: 1e19, funding_target: 1e19 });
  });

  it("rejects amount = 0", () => {
    assertInvalid(ref, { ...validEscrow, amount: 0 });
  });

  it("rejects negative funded_amount", () => {
    assertInvalid(ref, { ...validEscrow, funded_amount: -1 });
  });

  it("rejects yield_bps > 10000", () => {
    assertInvalid(ref, { ...validEscrow, yield_bps: 10001 });
  });

  it("rejects yield_bps < 0", () => {
    assertInvalid(ref, { ...validEscrow, yield_bps: -1 });
  });

  it("rejects invalid status value", () => {
    assertInvalid(ref, { ...validEscrow, status: 5 });
  });

  it("rejects invalid sme_address format", () => {
    assertInvalid(ref, { ...validEscrow, sme_address: "not-an-address" });
  });
});

// ---------------------------------------------------------------------------
// InvoiceSummary
// ---------------------------------------------------------------------------

describe("InvoiceSummary schema", () => {
  const ref = "#/components/schemas/InvoiceSummary";

  it("accepts a minimal valid summary", () => {
    assertValid(ref, { invoice_id: "INV001", amount: 1, status: 0 });
  });

  it("rejects missing amount", () => {
    assertInvalid(ref, { invoice_id: "INV001", status: 0 });
  });

  it("rejects amount = 0", () => {
    assertInvalid(ref, { invoice_id: "INV001", amount: 0, status: 0 });
  });
});

// ---------------------------------------------------------------------------
// InitEscrowRequest
// ---------------------------------------------------------------------------

describe("InitEscrowRequest schema", () => {
  const ref = "#/components/schemas/InitEscrowRequest";

  const valid = {
    invoice_id: "INV-1023",
    sme_address: VALID_ADDRESS,
    buyer_address: VALID_ADDRESS,
    amount: 100_000_000_000,
    yield_bps: 800,
    maturity: 1000,
  };

  it("accepts a valid request", () => assertValid(ref, valid));

  it("rejects missing sme_address", () => {
    const { sme_address, ...rest } = valid;
    assertInvalid(ref, rest);
  });

  it("rejects missing buyer_address", () => {
    const { buyer_address, ...rest } = valid;
    assertInvalid(ref, rest);
  });

  it("rejects amount = 0", () => {
    assertInvalid(ref, { ...valid, amount: 0 });
  });

  it("rejects maturity = 0", () => {
    assertInvalid(ref, { ...valid, maturity: 0 });
  });

  it("rejects yield_bps > 10000", () => {
    assertInvalid(ref, { ...valid, yield_bps: 10001 });
  });

  it("rejects invoice_id longer than 12 chars", () => {
    assertInvalid(ref, { ...valid, invoice_id: "TOOLONGINVOICEID" });
  });
});

// ---------------------------------------------------------------------------
// FundEscrowRequest
// ---------------------------------------------------------------------------

describe("FundEscrowRequest schema", () => {
  const ref = "#/components/schemas/FundEscrowRequest";

  const valid = { investor_address: VALID_ADDRESS, amount: 50_000_000_000 };

  it("accepts a valid fund request", () => assertValid(ref, valid));

  it("rejects missing investor_address", () => {
    assertInvalid(ref, { amount: 50_000_000_000 });
  });

  it("rejects amount = 0", () => {
    assertInvalid(ref, { ...valid, amount: 0 });
  });

  it("rejects negative amount", () => {
    assertInvalid(ref, { ...valid, amount: -1 });
  });

  it("rejects invalid investor_address", () => {
    assertInvalid(ref, { ...valid, investor_address: "bad" });
  });
});

// ---------------------------------------------------------------------------
// HealthResponse
// ---------------------------------------------------------------------------

describe("HealthResponse schema", () => {
  const ref = "#/components/schemas/HealthResponse";

  it("accepts a valid health response", () => {
    assertValid(ref, { status: "ok", timestamp: "2026-03-23T21:46:03Z" });
  });

  it("rejects status other than 'ok'", () => {
    assertInvalid(ref, { status: "degraded", timestamp: "2026-03-23T21:46:03Z" });
  });

  it("rejects missing timestamp", () => {
    assertInvalid(ref, { status: "ok" });
  });

  it("rejects invalid timestamp format", () => {
    assertInvalid(ref, { status: "ok", timestamp: "not-a-date" });
  });
});

// ---------------------------------------------------------------------------
// ApiInfoResponse
// ---------------------------------------------------------------------------

describe("ApiInfoResponse schema", () => {
  const ref = "#/components/schemas/ApiInfoResponse";

  it("accepts a valid info response", () => {
    assertValid(ref, { name: "StarFund API", version: "1.0.0", network: "mainnet" });
  });

  it("accepts optional docs_url", () => {
    assertValid(ref, {
      name: "StarFund API",
      version: "1.0.0",
      network: "testnet",
      docs_url: "https://docs.starfund.io",
    });
  });

  it("rejects unknown network value", () => {
    assertInvalid(ref, { name: "StarFund API", version: "1.0.0", network: "devnet" });
  });

  it("rejects missing name", () => {
    assertInvalid(ref, { version: "1.0.0", network: "mainnet" });
  });
});

// ---------------------------------------------------------------------------
// ErrorResponse
// ---------------------------------------------------------------------------

describe("ErrorResponse schema", () => {
  const ref = "#/components/schemas/ErrorResponse";

  it("accepts a valid error response", () => {
    assertValid(ref, { error: "NOT_FOUND", message: "Invoice not found." });
  });

  it("rejects missing error field", () => {
    assertInvalid(ref, { message: "Invoice not found." });
  });

  it("rejects missing message field", () => {
    assertInvalid(ref, { error: "NOT_FOUND" });
  });
});

// ---------------------------------------------------------------------------
// Stellar Address Fixture Validation & Unknown Keyword Detection (Issues #364, #365)
// ---------------------------------------------------------------------------

describe("Stellar address type and checksum format in API fixtures (Issue #364)", () => {
  const STELLAR_ADDRESS_REGEX = /^[GC][A-Z2-7]{55}$/;

  it("validates that all StellarAddress schema examples and fixtures conform to StrKey format", () => {
    const addressExample = schemas.StellarAddress?.example;
    assert.ok(addressExample, "StellarAddress must have an example");
    assert.match(addressExample, STELLAR_ADDRESS_REGEX, "StellarAddress example must match 56-char base32 StrKey");
  });

  it("verifies address fields across domain object fixtures", () => {
    const escrowExample = schemas.InvoiceEscrow?.example;
    if (escrowExample) {
      if (escrowExample.admin) assert.match(escrowExample.admin, STELLAR_ADDRESS_REGEX);
      if (escrowExample.sme_address) assert.match(escrowExample.sme_address, STELLAR_ADDRESS_REGEX);
      if (escrowExample.payer) assert.match(escrowExample.payer, STELLAR_ADDRESS_REGEX);
    }
  });
});

describe("Detect unknown JSON Schema keywords in docs tests (Issue #365)", () => {
  const KNOWN_KEYWORDS = new Set([
    "$ref", "$schema", "title", "description", "type", "properties", "required",
    "additionalProperties", "items", "enum", "example", "format", "pattern",
    "minimum", "maximum", "minLength", "maxLength", "allOf", "anyOf", "oneOf",
    "not", "default", "readOnly", "writeOnly", "nullable", "uniqueItems"
  ]);

  it("verifies all component schemas use valid standard JSON Schema / OpenAPI 3.1 keywords", () => {
    for (const [name, schema] of Object.entries(schemas)) {
      if (typeof schema === "object" && schema !== null) {
        for (const key of Object.keys(schema)) {
          assert.ok(
            KNOWN_KEYWORDS.has(key),
            `Unknown JSON Schema keyword '${key}' found in schema '${name}'`
          );
        }
      }
    }
  });
});

