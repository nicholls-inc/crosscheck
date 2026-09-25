-- DedupeTest.lean
-- runChecker reports a finding once, under the shortest path that reaches it.
-- Mirrors test_fixtures/plain_python/: record_invoice calls customer_label,
-- whose result is written to InvoiceRecord.customer.

import ContractGraph.Types
import ContractGraph.Main

namespace ContractGraphTest.DedupeTest

open ContractGraph

def recordInvoice : Node :=
  { id := 1, name := "record_invoice", kind := "function",
    preconditions := [], postconditions := [] }

def customerLabel : Node :=
  { id := 2, name := "customer_label", kind := "function",
    preconditions := []
    postconditions := [
      { kind := .length, staticBound := some 64,
        sourceFile := "pricing.py", sourceLine := 33, verificationLevel := .assumed }
    ] }

def customerField : Node :=
  { id := 3, name := "InvoiceRecord.customer", kind := "model",
    preconditions := [
      { kind := .length, staticBound := some 32,
        sourceFile := "records.py", sourceLine := 16, verificationLevel := .extracted }
    ]
    postconditions := [] }

def graph : ContractGraph :=
  { nodes := [recordInvoice, customerLabel, customerField]
    edges := [
      { source := recordInvoice, target := customerLabel, relationship := .calls },
      { source := customerLabel, target := customerField, relationship := .writesTo }
    ] }

def errorPaths (output : CheckOutput) : List (List String) :=
  (output.results.filter (·.severity == "error")).map (·.path)

-- Both paths (record_invoice → customer_label → field, and customer_label → field)
-- reach the same length violation; it is reported once, from its owner.
#guard errorPaths (runChecker graph) == [["customer_label", "InvoiceRecord.customer"]]
#guard (runChecker graph).exitCode == 1

-- Without deduplication both paths carry the error.
#guard ((collectResults (checkAllPaths graph)).filter (·.severity == "error")).length == 2

end ContractGraphTest.DedupeTest
