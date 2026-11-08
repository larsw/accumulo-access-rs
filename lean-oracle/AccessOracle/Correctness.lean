import AccessOracle.Spec
import AccessOracle.Parser

/-!
# The parser implements the specification

* `parse_iff`: `parse s = some x` exactly when the grammar derives `s` with value `x`. So the
  parser rejects every string the grammar does not derive, accepts every string it does, and
  returns the value the grammar assigns.
* `AccessExpression.unique`: the grammar assigns at most one value to a string.
* `accessEval_mono`: holding more authorizations never loses access.
* `nothing_only_empty`: an entity without authorizations can access only what an empty access
  expression protects (`SPECIFICATION.md`, "Evaluation").

The soundness and completeness proofs go by induction on the parser's fuel.
-/

namespace AccessOracle

/-! ## Characters -/

theorem isTokChar_quote : isTokChar '"' = false := by decide
theorem isTokChar_amp : isTokChar '&' = false := by decide
theorem isTokChar_bar : isTokChar '|' = false := by decide
theorem isTokChar_lparen : isTokChar '(' = false := by decide
theorem isTokChar_rparen : isTokChar ')' = false := by decide

theorem bar_ne_amp : ('|' : Char) ≠ '&' := by decide
theorem rparen_ne_amp : (')' : Char) ≠ '&' := by decide
theorem rparen_ne_bar : (')' : Char) ≠ '|' := by decide
theorem backslash_ne_quote : ('\\' : Char) ≠ '"' := by decide
theorem quote_ne_lparen : ('"' : Char) ≠ '(' := by decide

/-! ## What may follow -/

/-- What may follow an expression in a derivation: the end of the input or `)`. -/
def Follow (r : List Char) : Prop := r = [] ∨ ∃ r', r = ')' :: r'

/-- What may follow an operand: anything that does not start with an unquoted-token character,
which the parser would otherwise take as part of the token. -/
def NotTok (r : List Char) : Prop := ∀ c r', r = c :: r' → isTokChar c = false

theorem NotTok.cons {c : Char} {r : List Char} (h : isTokChar c = false) : NotTok (c :: r) := by
  intro c' r' hr
  cases hr
  exact h

theorem NotTok.of_follow {r : List Char} (h : Follow r) : NotTok r := by
  rcases h with rfl | ⟨r'', rfl⟩
  · intro c r' hr
    cases hr
  · exact NotTok.cons isTokChar_rparen

/-! ## Shapes of derivations -/

theorem Token.ne_nil {t v : List Char} (h : Token t v) : t ≠ [] := by
  cases h with
  | unquoted hne _ => exact hne
  | quoted _ _ => simp

/-- A token starts with a character other than `(`. -/
theorem Token.head {t v : List Char} (h : Token t v) : ∃ c t', t = c :: t' ∧ c ≠ '(' := by
  cases h with
  | unquoted hne hall =>
    cases t with
    | nil => exact absurd rfl hne
    | cons c t' =>
      refine ⟨c, t', rfl, ?_⟩
      intro hc
      subst hc
      have := hall '(' (by simp)
      rw [isTokChar_lparen] at this
      cases this
  | quoted _ _ => exact ⟨'"', _, rfl, quote_ne_lparen⟩

theorem Operand.ne_nil {t : List Char} {e : Expr} (h : Operand t e) : t ≠ [] := by
  cases h with
  | token hT => exact hT.ne_nil
  | paren _ => simp

theorem Expression.ne_nil {t : List Char} {e : Expr} (h : Expression t e) : t ≠ [] := by
  cases h with
  | operand hO => exact hO.ne_nil
  | conj hO _ => simp [hO.ne_nil]
  | disj hO _ => simp [hO.ne_nil]

theorem AndExpression.head {t : List Char} {e : Expr} (h : AndExpression t e) :
    ∃ t', t = '&' :: t' := by
  cases h <;> exact ⟨_, rfl⟩

theorem OrExpression.head {t : List Char} {e : Expr} (h : OrExpression t e) :
    ∃ t', t = '|' :: t' := by
  cases h <;> exact ⟨_, rfl⟩

/-! ## Tokens -/

theorem unquoted_append (cs r : List Char) (hcs : ∀ c ∈ cs, isTokChar c = true)
    (hr : NotTok r) : unquoted (cs ++ r) = (cs, r) := by
  induction cs with
  | nil =>
    cases r with
    | nil => rfl
    | cons c r' => simp [unquoted, hr c r' rfl]
  | cons c cs ih =>
    have hc : isTokChar c = true := hcs c (by simp)
    have := ih (fun x hx => hcs x (by simp [hx]))
    simp [unquoted, hc, this]

theorem unquoted_sound : ∀ (s tok r : List Char), unquoted s = (tok, r) →
    s = tok ++ r ∧ ∀ c ∈ tok, isTokChar c = true
  | [], tok, r, h => by
    simp [unquoted] at h
    obtain ⟨rfl, rfl⟩ := h
    simp
  | c :: rest, tok, r, h => by
    by_cases hc : isTokChar c = true
    · cases hu : unquoted rest with
      | mk tok' r' =>
        obtain ⟨heq, hall⟩ := unquoted_sound rest tok' r' hu
        simp [unquoted, hc, hu] at h
        obtain ⟨rfl, rfl⟩ := h
        refine ⟨by simp [heq], ?_⟩
        intro x hx
        simp at hx
        rcases hx with rfl | hx
        · exact hc
        · exact hall x hx
    · simp [unquoted, hc] at h
      obtain ⟨rfl, rfl⟩ := h
      simp

theorem quotedBody_plain {c : Char} (hc : isQuotedChar c = true) (rest : List Char) :
    quotedBody (c :: rest) = prepend c (quotedBody rest) := by
  simp [isQuotedChar] at hc
  simp [quotedBody, hc.1, hc.2]

theorem quotedBody_complete {raw v : List Char} (h : QuotedItems raw v) (r : List Char) :
    quotedBody (raw ++ '"' :: r) = some (v, r) := by
  induction h with
  | nil => simp [quotedBody]
  | cons hi _ ih =>
    cases hi with
    | plain hc => simp [quotedBody_plain hc, ih, prepend]
    | escapedQuote => simp [quotedBody, afterBackslash, backslash_ne_quote, ih, prepend]
    | escapedBackslash => simp [quotedBody, afterBackslash, backslash_ne_quote, ih, prepend]

theorem quotedBody_sound : ∀ (n : Nat) (s v r : List Char), s.length ≤ n →
    quotedBody s = some (v, r) → ∃ raw, s = raw ++ '"' :: r ∧ QuotedItems raw v := by
  intro n
  induction n with
  | zero =>
    intro s v r hs h
    cases s with
    | nil => simp [quotedBody] at h
    | cons _ _ => simp at hs
  | succ n ih =>
    intro s v r hs h
    cases s with
    | nil => simp [quotedBody] at h
    | cons c rest =>
      simp only [List.length_cons] at hs
      by_cases h1 : c = '"'
      · subst h1
        simp [quotedBody] at h
        obtain ⟨rfl, rfl⟩ := h
        exact ⟨[], rfl, .nil⟩
      · by_cases h2 : c = '\\'
        · subst h2
          cases rest with
          | nil => simp [quotedBody, afterBackslash, backslash_ne_quote] at h
          | cons d rest' =>
            simp only [List.length_cons] at hs
            by_cases hd : d = '"' ∨ d = '\\'
            · cases hq : quotedBody rest' with
              | none => simp [quotedBody, afterBackslash, backslash_ne_quote, hd, hq, prepend] at h
              | some p =>
                obtain ⟨v', r'⟩ := p
                obtain ⟨raw, hraw, hitems⟩ := ih rest' v' r' (by omega) hq
                simp [quotedBody, afterBackslash, backslash_ne_quote, hd, hq, prepend] at h
                obtain ⟨rfl, rfl⟩ := h
                have item : QuotedItem ['\\', d] d := by
                  rcases hd with rfl | rfl
                  · exact .escapedQuote
                  · exact .escapedBackslash
                exact ⟨['\\', d] ++ raw, by simp [hraw], .cons item hitems⟩
            · simp [quotedBody, afterBackslash, backslash_ne_quote, hd] at h
        · cases hq : quotedBody rest with
          | none => simp [quotedBody, h1, h2, hq, prepend] at h
          | some p =>
            obtain ⟨v', r'⟩ := p
            obtain ⟨raw, hraw, hitems⟩ := ih rest v' r' (by omega) hq
            simp [quotedBody, h1, h2, hq, prepend] at h
            obtain ⟨rfl, rfl⟩ := h
            have item : QuotedItem [c] c := .plain (by simp [isQuotedChar, h1, h2])
            exact ⟨[c] ++ raw, by simp [hraw], .cons item hitems⟩

theorem token_complete {t v r : List Char} (h : Token t v) (hr : NotTok r) :
    token (t ++ r) = some (v, r) := by
  cases h with
  | unquoted hne hall =>
    cases t with
    | nil => exact absurd rfl hne
    | cons c cs' =>
      have hc : isTokChar c = true := hall c (by simp)
      have hq : c ≠ '"' := by
        intro h
        subst h
        rw [isTokChar_quote] at hc
        cases hc
      have hu := unquoted_append (c :: cs') r hall hr
      simp only [List.cons_append] at hu ⊢
      simp [token, hq, hu]
  | quoted hitems hne =>
    have := quotedBody_complete hitems r
    simp [token, this, hne]

theorem token_sound {s v r : List Char} (h : token s = some (v, r)) :
    ∃ t, s = t ++ r ∧ Token t v := by
  cases s with
  | nil => simp [token] at h
  | cons c rest =>
    by_cases hc : c = '"'
    · subst hc
      cases hq : quotedBody rest with
      | none => simp [token, hq] at h
      | some p =>
        obtain ⟨v', r'⟩ := p
        obtain ⟨raw, hraw, hitems⟩ := quotedBody_sound rest.length rest v' r' (Nat.le_refl _) hq
        simp [token, hq] at h
        obtain ⟨hne, rfl, rfl⟩ := h
        exact ⟨'"' :: (raw ++ ['"']), by simp [hraw], .quoted hitems hne⟩
    · cases hu : unquoted (c :: rest) with
      | mk tok r' =>
        obtain ⟨heq, hall⟩ := unquoted_sound _ _ _ hu
        simp [token, hc, hu] at h
        obtain ⟨hne, rfl, rfl⟩ := h
        exact ⟨_, heq, .unquoted hne hall⟩

/-! ## Expressions -/

/-- Whatever the parser returns is derived by the grammar. -/
theorem sound : ∀ n : Nat,
    (∀ s e r, expression n s = some (e, r) → ∃ t, s = t ++ r ∧ Expression t e) ∧
    (∀ s e r, operand n s = some (e, r) → ∃ t, s = t ++ r ∧ Operand t e) ∧
    (∀ s e r, andTail n s = some (e, r) → ∃ t, s = t ++ r ∧ AndExpression ('&' :: t) e) ∧
    (∀ s e r, orTail n s = some (e, r) → ∃ t, s = t ++ r ∧ OrExpression ('|' :: t) e) := by
  intro n
  induction n with
  | zero => simp [expression, operand, andTail, orTail]
  | succ n ih =>
    obtain ⟨ihE, ihO, ihA, ihR⟩ := ih
    refine ⟨?_, ?_, ?_, ?_⟩
    · -- expression
      intro s e r h
      cases ho : operand n s with
      | none => simp [expression, ho] at h
      | some p =>
        obtain ⟨e₀, r₀⟩ := p
        obtain ⟨t₀, hs, hO⟩ := ihO s e₀ r₀ ho
        cases r₀ with
        | nil =>
          simp [expression, ho] at h
          obtain ⟨rfl, rfl⟩ := h
          exact ⟨t₀, hs, .operand hO⟩
        | cons c r' =>
          by_cases h1 : c = '&'
          · subst h1
            cases ha : andTail n r' with
            | none => simp [expression, ho, ha] at h
            | some q =>
              obtain ⟨e₁, r₁⟩ := q
              obtain ⟨t₁, ht, hA⟩ := ihA r' e₁ r₁ ha
              simp [expression, ho, ha] at h
              obtain ⟨rfl, rfl⟩ := h
              exact ⟨t₀ ++ '&' :: t₁, by simp [hs, ht], .conj hO hA⟩
          · by_cases h2 : c = '|'
            · subst h2
              cases ha : orTail n r' with
              | none => simp [expression, ho, ha, bar_ne_amp] at h
              | some q =>
                obtain ⟨e₁, r₁⟩ := q
                obtain ⟨t₁, ht, hR⟩ := ihR r' e₁ r₁ ha
                simp [expression, ho, ha, bar_ne_amp] at h
                obtain ⟨rfl, rfl⟩ := h
                exact ⟨t₀ ++ '|' :: t₁, by simp [hs, ht], .disj hO hR⟩
            · simp [expression, ho, h1, h2] at h
              obtain ⟨rfl, rfl⟩ := h
              exact ⟨t₀, hs, .operand hO⟩
    · -- operand
      intro s e r h
      cases s with
      | nil => simp [operand] at h
      | cons c rest =>
        by_cases hc : c = '('
        · subst hc
          cases hx : expression n rest with
          | none => simp [operand, hx] at h
          | some p =>
            obtain ⟨e₀, r₀⟩ := p
            cases r₀ with
            | nil => simp [operand, hx] at h
            | cons d r' =>
              by_cases hd : d = ')'
              · subst hd
                obtain ⟨t₀, hs, hE⟩ := ihE rest e₀ (')' :: r') hx
                simp [operand, hx] at h
                obtain ⟨rfl, rfl⟩ := h
                exact ⟨'(' :: (t₀ ++ [')']), by simp [hs], .paren hE⟩
              · simp [operand, hx, hd] at h
        · cases ht : token (c :: rest) with
          | none => simp [operand, hc, ht] at h
          | some p =>
            obtain ⟨v, r'⟩ := p
            obtain ⟨t, hs, hT⟩ := token_sound ht
            simp [operand, hc, ht] at h
            obtain ⟨rfl, rfl⟩ := h
            exact ⟨t, hs, .token hT⟩
    · -- andTail
      intro s e r h
      cases ho : operand n s with
      | none => simp [andTail, ho] at h
      | some p =>
        obtain ⟨e₀, r₀⟩ := p
        obtain ⟨t₀, hs, hO⟩ := ihO s e₀ r₀ ho
        cases r₀ with
        | nil =>
          simp [andTail, ho] at h
          obtain ⟨rfl, rfl⟩ := h
          exact ⟨t₀, hs, .last hO⟩
        | cons c r' =>
          by_cases h1 : c = '&'
          · subst h1
            cases ha : andTail n r' with
            | none => simp [andTail, ho, ha] at h
            | some q =>
              obtain ⟨e₁, r₁⟩ := q
              obtain ⟨t₁, ht, hA⟩ := ihA r' e₁ r₁ ha
              simp [andTail, ho, ha] at h
              obtain ⟨rfl, rfl⟩ := h
              exact ⟨t₀ ++ '&' :: t₁, by simp [hs, ht], .more hO hA⟩
          · simp [andTail, ho, h1] at h
            obtain ⟨rfl, rfl⟩ := h
            exact ⟨t₀, hs, .last hO⟩
    · -- orTail
      intro s e r h
      cases ho : operand n s with
      | none => simp [orTail, ho] at h
      | some p =>
        obtain ⟨e₀, r₀⟩ := p
        obtain ⟨t₀, hs, hO⟩ := ihO s e₀ r₀ ho
        cases r₀ with
        | nil =>
          simp [orTail, ho] at h
          obtain ⟨rfl, rfl⟩ := h
          exact ⟨t₀, hs, .last hO⟩
        | cons c r' =>
          by_cases h1 : c = '|'
          · subst h1
            cases ha : orTail n r' with
            | none => simp [orTail, ho, ha] at h
            | some q =>
              obtain ⟨e₁, r₁⟩ := q
              obtain ⟨t₁, ht, hR⟩ := ihR r' e₁ r₁ ha
              simp [orTail, ho, ha] at h
              obtain ⟨rfl, rfl⟩ := h
              exact ⟨t₀ ++ '|' :: t₁, by simp [hs, ht], .more hO hR⟩
          · simp [orTail, ho, h1] at h
            obtain ⟨rfl, rfl⟩ := h
            exact ⟨t₀, hs, .last hO⟩

/-- Given enough fuel, the parser returns what the grammar derives, provided that what follows
could follow it in a derivation. -/
theorem complete : ∀ n : Nat,
    (∀ t e r, Expression t e → Follow r → t.length < n →
      expression n (t ++ r) = some (e, r)) ∧
    (∀ t e r, Operand t e → NotTok r → t.length ≤ n →
      operand n (t ++ r) = some (e, r)) ∧
    (∀ t e r, AndExpression ('&' :: t) e → Follow r → t.length < n →
      andTail n (t ++ r) = some (e, r)) ∧
    (∀ t e r, OrExpression ('|' :: t) e → Follow r → t.length < n →
      orTail n (t ++ r) = some (e, r)) := by
  intro n
  induction n with
  | zero =>
    refine ⟨?_, ?_, ?_, ?_⟩
    · intro t e r _ _ h
      omega
    · intro t e r hO _ h
      cases t with
      | nil => exact absurd rfl hO.ne_nil
      | cons _ _ => simp at h
    · intro t e r _ _ h
      omega
    · intro t e r _ _ h
      omega
  | succ n ih =>
    obtain ⟨ihE, ihO, ihA, ihR⟩ := ih
    refine ⟨?_, ?_, ?_, ?_⟩
    · -- expression
      intro t e r hE hF hlen
      cases hE with
      | operand hO =>
        have h1 := ihO _ _ r hO (NotTok.of_follow hF) (by omega)
        simp only [expression]
        rw [h1]
        rcases hF with rfl | ⟨r', rfl⟩
        · rfl
        · simp [rparen_ne_amp, rparen_ne_bar]
      | @conj s u e₀ e₁ hO hA =>
        obtain ⟨u', rfl⟩ := hA.head
        simp only [List.length_append, List.length_cons] at hlen
        have h1 := ihO s e₀ ('&' :: (u' ++ r)) hO (NotTok.cons isTokChar_amp) (by omega)
        have h2 := ihA u' e₁ r hA hF (by omega)
        rw [List.append_assoc, List.cons_append]
        simp only [expression]
        rw [h1]
        simp [h2]
      | @disj s u e₀ e₁ hO hR =>
        obtain ⟨u', rfl⟩ := hR.head
        simp only [List.length_append, List.length_cons] at hlen
        have h1 := ihO s e₀ ('|' :: (u' ++ r)) hO (NotTok.cons isTokChar_bar) (by omega)
        have h2 := ihR u' e₁ r hR hF (by omega)
        rw [List.append_assoc, List.cons_append]
        simp only [expression]
        rw [h1]
        simp [h2, bar_ne_amp]
    · -- operand
      intro t e r hO hN hlen
      cases hO with
      | @token s v hT =>
        obtain ⟨c, s', rfl, hc⟩ := hT.head
        have h1 := token_complete hT hN
        simp only [List.cons_append] at h1 ⊢
        simp [operand, hc, h1]
      | paren hE =>
        simp only [List.length_cons, List.length_append, List.length_nil] at hlen
        have h1 := ihE _ _ (')' :: r) hE (Or.inr ⟨r, rfl⟩) (by omega)
        simp only [List.cons_append, List.append_assoc, List.nil_append]
        simp only [operand]
        rw [h1]
        simp
    · -- andTail
      intro t e r hA hF hlen
      cases hA with
      | last hO =>
        have h1 := ihO _ _ r hO (NotTok.of_follow hF) (by omega)
        simp only [andTail]
        rw [h1]
        rcases hF with rfl | ⟨r', rfl⟩
        · rfl
        · simp [rparen_ne_amp]
      | @more s u e₀ e₁ hO hA' =>
        obtain ⟨u', rfl⟩ := hA'.head
        simp only [List.length_append, List.length_cons] at hlen
        have h1 := ihO s e₀ ('&' :: (u' ++ r)) hO (NotTok.cons isTokChar_amp) (by omega)
        have h2 := ihA u' e₁ r hA' hF (by omega)
        rw [List.append_assoc, List.cons_append]
        simp only [andTail]
        rw [h1]
        simp [h2]
    · -- orTail
      intro t e r hR hF hlen
      cases hR with
      | last hO =>
        have h1 := ihO _ _ r hO (NotTok.of_follow hF) (by omega)
        simp only [orTail]
        rw [h1]
        rcases hF with rfl | ⟨r', rfl⟩
        · rfl
        · simp [rparen_ne_bar]
      | @more s u e₀ e₁ hO hR' =>
        obtain ⟨u', rfl⟩ := hR'.head
        simp only [List.length_append, List.length_cons] at hlen
        have h1 := ihO s e₀ ('|' :: (u' ++ r)) hO (NotTok.cons isTokChar_bar) (by omega)
        have h2 := ihR u' e₁ r hR' hF (by omega)
        rw [List.append_assoc, List.cons_append]
        simp only [orTail]
        rw [h1]
        simp [h2]

/-! ## Access expressions -/

theorem parse_sound {s : List Char} {x : Option Expr} (h : parse s = some x) :
    AccessExpression s x := by
  cases s with
  | nil =>
    simp [parse] at h
    subst h
    exact .empty
  | cons c rest =>
    cases hx : expression (rest.length + 2) (c :: rest) with
    | none => simp [parse, hx] at h
    | some p =>
      obtain ⟨e, r⟩ := p
      cases r with
      | cons _ _ => simp [parse, hx] at h
      | nil =>
        obtain ⟨t, hs, hE⟩ := (sound _).1 _ _ _ hx
        simp [parse, hx] at h
        subst h
        simp only [List.append_nil] at hs
        subst hs
        exact .expression hE

theorem parse_complete {s : List Char} {x : Option Expr} (h : AccessExpression s x) :
    parse s = some x := by
  cases h with
  | empty => rfl
  | @expression s e hE =>
    cases s with
    | nil => exact absurd rfl hE.ne_nil
    | cons c rest =>
      have h1 := (complete (rest.length + 2)).1 (c :: rest) e [] hE (Or.inl rfl)
        (by simp only [List.length_cons]; omega)
      simp only [List.append_nil] at h1
      simp [parse, h1]

/-- The parser accepts `s` with value `x` exactly when the grammar derives `s` with value `x`. -/
theorem parse_iff {s : List Char} {x : Option Expr} :
    parse s = some x ↔ AccessExpression s x :=
  ⟨parse_sound, parse_complete⟩

/-- The parser rejects exactly the strings the grammar does not derive. -/
theorem parse_none_iff {s : List Char} : parse s = none ↔ ∀ x, ¬AccessExpression s x := by
  constructor
  · intro h x hx
    rw [parse_complete hx] at h
    cases h
  · intro h
    cases hp : parse s with
    | none => rfl
    | some x => exact absurd (parse_sound hp) (h x)

/-- The grammar is unambiguous: it derives each string with at most one value. -/
theorem AccessExpression.unique {s : List Char} {x y : Option Expr}
    (hx : AccessExpression s x) (hy : AccessExpression s y) : x = y :=
  Option.some.inj ((parse_complete hx).symm.trans (parse_complete hy))

/-! ## Evaluation -/

theorem Expr.eval_mono {a b : List Char → Bool} (hab : ∀ v, a v = true → b v = true)
    (e : Expr) : e.eval a = true → e.eval b = true := by
  induction e with
  | tok v =>
    simp only [Expr.eval]
    exact hab v
  | conj l r ihl ihr =>
    simp only [Expr.eval, Bool.and_eq_true]
    exact fun ⟨h1, h2⟩ => ⟨ihl h1, ihr h2⟩
  | disj l r ihl ihr =>
    simp only [Expr.eval, Bool.or_eq_true]
    exact fun h => h.elim (fun h => .inl (ihl h)) (fun h => .inr (ihr h))

/-- Holding more authorizations never loses access. -/
theorem accessEval_mono {a b : List Char → Bool} (hab : ∀ v, a v = true → b v = true)
    (x : Option Expr) : accessEval a x = true → accessEval b x = true := by
  cases x with
  | none => simp [accessEval]
  | some e => exact Expr.eval_mono hab e

theorem Expr.eval_nothing (e : Expr) : e.eval (fun _ => false) = false := by
  induction e with
  | tok v => rfl
  | conj l r ihl _ => simp [Expr.eval, ihl]
  | disj l r ihl ihr => simp [Expr.eval, ihl, ihr]

/-- "An entity with empty Authorizations can only access data associated with an empty access
expression" (`SPECIFICATION.md`, "Evaluation"). -/
theorem nothing_only_empty {s : List Char} {x : Option Expr} (h : parse s = some x) :
    accessEval (fun _ => false) x = true ↔ s = [] := by
  cases parse_sound h with
  | empty => simp [accessEval]
  | expression hE => simp [accessEval, Expr.eval_nothing, hE.ne_nil]

end AccessOracle
