/-!
# Exact Z3 pump algebra for a protected non-Abelian acoustic target

This finite model records the endpoint action of the two Z3 acoustic pump
cycles used by the reference instrument.  It proves order-three closure,
noncommutativity, and a pumped-versus-trapped sequence witness.

It does not prove the second Chern invariant, a spectral gap, adiabaticity, or
the correspondence between this finite action and a physical acoustic
waveguide.  Those remain analytic and numerical certificates.
-/

namespace MusFormal.ProtectedNonAbelian

inductive Cell
  | left
  | center
  | right
  deriving DecidableEq, Repr

def shiftLeft : Cell → Cell
  | .left => .right
  | .center => .left
  | .right => .center

def shiftRight : Cell → Cell
  | .left => .center
  | .center => .right
  | .right => .left

theorem shifts_inverse_left (cell : Cell) :
    shiftRight (shiftLeft cell) = cell := by
  cases cell <;> rfl

theorem shifts_inverse_right (cell : Cell) :
    shiftLeft (shiftRight cell) = cell := by
  cases cell <;> rfl

inductive PumpMode
  | plus
  | minus
  | link
  deriving DecidableEq, Repr

structure PumpState where
  cell : Cell
  mode : PumpMode
  deriving DecidableEq, Repr

/-- The within-cell Z3 cycle: plus -> link -> minus -> plus. -/
def c3 : PumpState → PumpState
  | ⟨cell, .plus⟩ => ⟨cell, .link⟩
  | ⟨cell, .link⟩ => ⟨cell, .minus⟩
  | ⟨cell, .minus⟩ => ⟨cell, .plus⟩

/-- The translated Z3 cycle:
    plus_N -> minus_N -> link_(N-1) -> plus_N. -/
def c3Prime : PumpState → PumpState
  | ⟨cell, .plus⟩ => ⟨cell, .minus⟩
  | ⟨cell, .minus⟩ => ⟨shiftLeft cell, .link⟩
  | ⟨cell, .link⟩ => ⟨shiftRight cell, .plus⟩

theorem c3_order_three (state : PumpState) :
    c3 (c3 (c3 state)) = state := by
  cases state with
  | mk cell mode =>
      cases mode <;> rfl

theorem c3Prime_order_three (state : PumpState) :
    c3Prime (c3Prime (c3Prime state)) = state := by
  cases state with
  | mk cell mode =>
      cases mode <;> simp [c3Prime, shifts_inverse_left, shifts_inverse_right]

def initial : PumpState := ⟨.center, .plus⟩

/-- The two pump generators are noncommuting already on one basis state. -/
theorem pump_generators_noncommute :
    c3 (c3Prime initial) ≠ c3Prime (c3 initial) := by
  decide

/-- Chronological word C3,C3,C3',C3. -/
def pathOne : PumpState → PumpState :=
  fun state => c3 (c3Prime (c3 (c3 state)))

/-- Chronological word C3',C3,C3,C3. -/
def pathTwo : PumpState → PumpState :=
  fun state => c3 (c3 (c3 (c3Prime state)))

theorem path_one_pumps :
    pathOne initial = ⟨.left, .minus⟩ := by
  rfl

theorem path_two_traps :
    pathTwo initial = ⟨.center, .minus⟩ := by
  rfl

theorem path_order_changes_transport :
    pathOne initial ≠ pathTwo initial := by
  decide

end MusFormal.ProtectedNonAbelian
