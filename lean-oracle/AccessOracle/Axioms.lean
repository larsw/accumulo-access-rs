import AccessOracle.Correctness

/-!
# Axioms

`lake build` only warns when a proof uses `sorry`, and says nothing about `native_decide`, which
trusts the compiler. This fails the build if a theorem in `Correctness.lean` comes to depend on any
axiom beyond the ones listed, such as `sorryAx` or `Lean.ofReduceBool`.
-/

namespace AccessOracle

/-- info: 'AccessOracle.parse_iff' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in #print axioms parse_iff

/-- info: 'AccessOracle.parse_none_iff' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in #print axioms parse_none_iff

/-- info: 'AccessOracle.AccessExpression.unique' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in #print axioms AccessExpression.unique

/-- info: 'AccessOracle.accessEval_mono' depends on axioms: [propext] -/
#guard_msgs in #print axioms accessEval_mono

/-- info: 'AccessOracle.nothing_only_empty' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in #print axioms nothing_only_empty

end AccessOracle
