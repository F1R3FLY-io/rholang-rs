# Cost-Accounted Rholang — Parser Syntax

> The surface syntax for cost-accounted Rholang, added to the Rholang
> tree-sitter grammar + CPS parser. The normalizer that lowers it to stable
> Rholang (`Par`) lives in the f1r3node worktree
> (`f1r3node-rust-cost-accounting-transpiler`), documented in
> `docs/cost-accounting/transpiler.md` there.

**Provenance.** The surface syntax is now **authoritative**: it tracks Greg
(Lucius Meredith)'s appendix *"Concrete Syntax for Rholang 1.2"* in
`../publications/cost-accounting/cost-accounted-rho.tex@fdf089e`
(`\label{app:concrete}`), which makes the desired ASCII spellings explicit as
LBNF/BNFC productions. An earlier *collision-safe scratch rendering* (derived
from the abstract `mettail-rust-cost-accounting` notation `{p}_s` / `s:S` /
`g ∣ #P ∣ s∘s`) has been superseded; the constructs below differ from it in
five places, all aligned here: `# P` (no parens), `s1 (*) s2` (distinct
lexeme), bare stacks (no `purse(...)`), ring-fencing via `new`-bound signatures
(no located purse), and the polymorphic *SignedProc* forms (signed sends,
signed `for`-continuations, per-clause signed binds). The grammar was confirmed
conflict-free and **ABI-stable** (`LANGUAGE_VERSION 15`) by the `tree-sitter
generate` spike (pinned CLI `0.25.6`).

---

## 1. Concrete syntax (Greg `app:concrete`)

| Construct | Abstract | Surface | Notes |
|---|---|---|---|
| Signed term | `{P}_s` | `{% P %}[ s ]` | `P` any process; `s` a signature |
| Token cons | `s : S` | `s :: S` | `::` right-assoc; tail is `()` |
| Empty stack | `()` | `()` | reuses `unit`/`Nil` |
| Stack-as-process | `S` | bare `s :: ()` | a stack is **directly** a process — no `purse(...)` |
| Located / parallel stacks | `S1 ∥ S2` | `S1 \| S2` | ordinary parallel composition (no located form) |
| Ground signature | `g` | `s` | a bare identifier (`_proc_var`) |
| Section / hash | `#P` | `# P` | `#` then a `Proc12` principal, **no parens** |
| Compound signature | `s1 ∘ s2` | `s1 (*) s2` | `(*)` a distinct lexeme; left-assoc |
| Lollipop signature | `s1 ⊸ s2` | `s1 -o s2` | right-assoc (sugar) |
| Signed send | `x!(U)` | `x!( {% P %}[s] )`, `x!( s :: () )` | payload is any `_proc` |
| Signed `for`-continuation | `for(y<-x){T}` | `for(...) {% P %}` | continuation wrapped, **no** `[s]` |
| Per-clause signed bind | `{y<-x}_s` | `{% y <- x %}[ s ]` | Axis-C join; a *bind*, not a *proc* |

**Signature precedence** (stratified per Greg's BNFC):

```
Sig  ::= Sig1 -o Sig  |  Sig1          -- lollipop, loosest, right-assoc
Sig1 ::= Sig1 (*) Sig2 |  Sig2          -- compound, tighter, left-assoc
Sig2 ::= g  |  # Proc12                 -- ground / section, tightest
```

So `# Nil (*) b -o c` parses as `Transfer(Compound(Hash, Ground b), Ground c)`
— `(# Nil)` binds tightest, then `(*)`, then `-o`.

### Tokenization notes (`tree-sitter generate` spike)

* **Literal tokens:** `{%`, `%}`, `::`, `#`, `(*)`, `-o`. The keyword `purse` is
  **gone** (a bare stack is a process; ring-fencing moved to `new`-bound sigs).
* **`-o` confinement.** `-o` lives ONLY at the top `Sig`, reachable only inside
  the `[ … ]` of a signed term / signed bind. It never enters the bare-process
  lexer state, so `x-owed` still lexes as `x - owed` (`sub(x, owed)`), and
  `z--owed` as `diff(z, owed)`. No external scanner needed. Token-stack layers
  use the transfer-free `_sig1` (a lollipop is not a fundable atom — the
  normalizer rejects `Transfer` in a stack).
* **`(*)` is a distinct lexeme**, never `(` `*` `)`. Bare `*` stays dereference /
  multiply; this is why the compound uses `(*)` and not `*`.
* **`# P` without parens** binds a high-precedence principal (`_ground_expression`
  / `eval` — the Rholang `Proc12` level: ground terms, collections, `*x`,
  parenthesized). Hash a complex process with `# { P }`.
* **Bare stack vs map.** `s :: S` (proc-var head + `::`) does not collide with a
  map's `key : value` (single `:` ≠ double `::`); the empty `()` is plain
  `unit`/`Nil`.
* **The three `{%`-forms** disambiguate by context + the trailing `[s]`:
  `{% P %}[s]` (signed term, a *proc*, trailing `[s]`); `{% P %}` (signed
  `for`-continuation, a *proc*, no `[s]`, only as a `for` body); `{% y<-x %}[s]`
  (per-clause signed bind, a *bind*, only inside `for(...)`'s receipt).

---

## 2. AST (`rholang-parser/src/ast.rs`)

```rust
Proc::SignedTerm { proc: AnnProc<'ast>, sig: Signature<'ast> }
Proc::TokenStack { stack: TokenStack<'ast> }          // bare stack `s :: … :: ()` — no `id`

enum Signature<'ast> {
    Ground(Name<'ast>),                                   // bare name (a `_proc_var`)
    Hash(AnnProc<'ast>),                                  // # P
    Compound(Box<Signature<'ast>>, Box<Signature<'ast>>), // s1 (*) s2
    Transfer(Box<Signature<'ast>>, Box<Signature<'ast>>), // s1 -o s2 (sugar; desugared in the normalizer)
}

struct TokenStack<'ast> { layers: SmallVec<[Signature<'ast>; 2]> }

enum Bind<'ast> {
    Linear   { lhs: Names<'ast>, rhs: Source<'ast> },
    Repeated { lhs: Names<'ast>, rhs: Name<'ast> },
    Peek     { lhs: Names<'ast>, rhs: Name<'ast> },
    Signed   { lhs: Names<'ast>, rhs: Source<'ast>, sig: Signature<'ast> },  // {% y<-x %}[s]
}
```

* **Located purses are gone.** `Proc::Purse { id, stack }` became
  `Proc::TokenStack { stack }` (no identity field). A located/parallel stack is
  just `S1 | S2` (ordinary `Par`); ring-fencing is realised by a **`new`-bound
  signature** (the binding-sensitive `Σ⟦s⟧` in the normalizer — see
  `transpiler.md`), not by a syntactic owner.
* **Signed sends** need no new node: once `signed_term`/`token_stack` are
  processes, `x!( {% P %}[s] )` and `x!( s :: () )` parse as an ordinary `Send`
  with a cost-bearing payload.
* **Signed `for`-continuations** need no new node: `for(...) {% P %}` parses with
  the `{% … %}` wrapper, which the CPS parser unwraps to the inner `P` (it marks
  the continuation as a *SignedProc*; any signing is carried by signed terms /
  binds inside `P`).
* **Per-clause signed binds** add the `Bind::Signed` variant; N-ary joins still
  reuse `Proc::ForComprehension` (signed and plain binds compose with `&`).

---

## 3. CPS parser (`rholang-parser/src/parser/parsing.rs`)

A flat `SigOp` descriptor (`Ground | Hash | Compound | Transfer`) with
`flatten_signature` (post-order) + `rebuild_signature` (fold) reconstructs the
`Signature` tree. `flatten_signature` is **wrapper-robust**: it transparently
descends the named wrappers (`signature`, `stack_sig`) and the hidden
stratification levels (`_sig1`, `_sig2`) before dispatching on
`ground`/`hash`/`compound`/`transfer`. Continuations `K::ConsumeSignedTerm` /
`K::ConsumeTokenStack` assemble the proc nodes; `signed_cont` is unwrapped in
place (its inner `proc` field replaces the node), and `signed_bind` is threaded
through `BindDesc::Signed` (its slots: `[source_name, SR-inputs, names…,
sig_procs…]`). `rebuild_signature`'s ground case falls back to `Name::Quote`
for a non-`ProcVar` (malformed input is rejected downstream).

## 4. Traversal + downstream (`traverse.rs`, `rholang-lib`)

`Signature` is a sub-sort reachable by no existing traversal, so the three
iterators (`PreorderDfsIter`, `DfsEventIter`, `NameAwareDfsEventIter`) descend
into `Hash` bodies + quoted ground names via a `signature_proc_children` helper.
The semantic-index coupling invariant holds: every node `iter_preorder_dfs`
yields gets a PID.

`rholang-lib` (the semantic-analysis crate, used by tooling/LSP) rejects cost
syntax in pattern position with `ErrorKind::CostSyntaxInsidePattern`
(resolver + elaborator). **Note:** f1r3node's normalizer does NOT depend on
`rholang-lib`, so it re-implements that rejection in its own `pattern_guard`.
The MVP `rholang-compiler` codegen rejects `Bind::Signed` (cost-accounting is
lowered by the f1r3node normalizer, not the MVP path).

---

## 5. Tests

* `rholang-tree-sitter/test/corpus/cost_accounting.txt` — CST corpus
  (re-spelled to Greg `app:concrete`; full corpus 90/90).
* `rholang-parser/tests/cost_accounting_tests.rs` — AST-level (18 cases):
  precedence/associativity, compound `(*)`, `# { P }`, bare stacks, multi-layer
  stacks, send payloads, `Bind::Signed`, `signed_cont`, ring-fence-via-`new`,
  `x-owed` subtraction (no `-o` mis-lex), malformed-rejection, joins regression.
* `rholang-parser/tests/corpus/cost_*.rho` — golden snapshot fixtures
  (re-spelled; `cost_signed_bind.rho` added).
* Golden snapshots remain ABI-stable (`LANGUAGE_VERSION 15`).
