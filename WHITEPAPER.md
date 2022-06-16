Kindelia: a peer-to-peer functional computer
============================================

Turing-complete blockchains, such as Ethereum, allow users to deploy
decentralized applications (DApps), but recurrent security exploits undermine
their proposals. Formal verification techniques can be used to ensure a DApp is
exploit-free, but Ethereum's underlying processor, the EVM, make these too
expensive to be viable. By leveraging a functional virtual machine, the
[HVM](https://github.com/kindelia/hvm), Kindelia is able to run formally
verified DApps cheaply and efficiently, making it the most secure peer-to-peer
computer in existence. Moreover, by storing the global state as reversible
runtime heaps, it can run highly dynamic applications with massively reduced
costs, making layer 1 virtual worlds economically viable.  Finally, there is no
built-in token, and, thus, **it isn't a cryptocurrency**.  Instead, Kindelia is
merely a peer-to-peer functional computer capable of efficiently hosting
hack-proof programs that can't ever be turned off.

Table of Contents
=================

* [Introduction](#introduction)
* [Examples](#examples)
  * [Block #1: defining pure functions](#block-1-defining-pure-functions)
  * [Block #2: defining stateful functions](#block-2-defining-stateful-functions)
  * [Block #3: signing statements](#block-3-signing-statements)
  * [Block #4: playing the game](#block-4-playing-the-game)
* [Technical Overview](#technical-overview)
  * [Blocks and Statements](#blocks-and-statements)
  * [Expressions](#expressions)
  * [IO Effects](#io-effects)
  * [Computation Rules](#computation-rules)
  * [Memory Model](#memory-model)
  * [Genesis Block](#genesis-block)
  * [Table of Costs](#table-of-costs)
  * [Serialization](#serialization)
* [The High-order Virtual Machine (HVM)](#the-high-order-virtual-machine-hvm)
  * [Sequential tasks: as fast as Haskell's GHC](#1-for-normal-sequential-tasks-such-as-folds-it-holds-similar-performance)
  * [Parallel tasks: several times faster](#2-for-very-parallel-tasks-such-as-tree-based-quicksort-it-is-several-times-faster)
  * [High-order tasks: exponentially faster](#3-for-very-high-order-tasks-it-is-exponentially-faster)
  * [Negligible compilation times](#1-it-has-negligible-compliation-times)
  * [Measurable computation costs](#2-computation-costs-are-measurable-including-space-and-time)
  * [How is that possible?](how-is-that-possible)
  * [Is Kindelia a consequence of the HVM?](is-kindelia-a-consequence-of-the-hvm)
* [Comparisons to Ethereum](#comparisons-to-ethereum)
  * [Functional opcodes, formally verified apps](#hvm-makes-functional-and-formally-verified-dapps-much-cheaper)
  * [Zero-cost SSTOREs, highly dynamic apps](#reversible-heaps-make-dynamic-dapps-much-cheaper)
  * [Massive layer-1 optimizations](#optimizations-and-simplifications-everywhere)
  * [Comparison table](#comparison-table)
  * [Summary](#in-short)
* [Comparisons to Cardano](#comparisons-to-cardano)
  * [On security](#on-security)
  * [On efficiency](#on-efficiency)
  * [On simplicity](#on-simplicity)
  * [Summary](#in-short)
* [Whys](#whys)
  * [Why create a cryptocurrency?](#why-create-a-cryptocurrency)
  * [Why not include a currency?](#why-not-include-a-currency)
  * [Why Nakamoto Consensus (Proof of Work)?](#why-nakamoto-consensus-proof-of-work)
  * [Why not become a layer 2 Ethereum rollup?](#why-not-become-a-layer-2-ethereum-rollup)

Introduction
============

Bitcoin was released in 2009, and became the first successful decentralized
program. Soon enough, forks emerged aiming to create special-purpose coins such
as NameCoin, which allowed purchasing and selling names, and ColorCoin, which
featured multiple tokens. In 2013, Ethereum generalized this concept by
including a stateful, Turing complete virtual machine. Arbitrary apps could be
easily created as Ethereum contracts, without launching a new network. More than
a decentralized application, Ethereum was the first decentralized computer. Due
to its quick development, though, it launched with several design mistakes that
made its base layer considerably less scalable and secure than it could be. In
2021, the average Ethereum transaction fee rose to as much as 70 USD [citation](https://markets.businessinsider.com/news/currencies/ethereum-transaction-gas-fees-high-solana-avalanche-cardano-crypto-blockchain-2021-12#:~:text=The%20average%20transaction%20or%20%22gas,to%20crypto%20exchange%20Kraken's%20analysts.),
and millionaire smart-contract exploits were so common that websites reported
their ocurrences daily [citation](https://www.theregister.com/2022/04/26/smart_contract_losses/).

- Regarding **scalability**, layer 2 techniques are proposed as a solution, but
  these come with their own compromises. There is value in a more scalable
  layer 1, but Ethereum's historical mistakes leave little room for improvement.

- Regarding **security**, formal verification techniques allow developers to
  deploy contracts with zero bugs on their first version, which is invaluable,
  since they can't be patched or reversed, but the EVM makes these prohibitively
  expensive [citation].

Kindelia is a complete redesign of Ethereum's base layer. It replaces the EVM by
the [HVM](https://github.com/kindelia/hvm), a fast functional virtual machine
that greatly decreases the cost of ultra-safe formally verified contracts and
highly-dynamic apps such as virtual RPG worlds, and a wide range of
optimizations and polishments make it the most stable, efficient and secure
layer 1 computer in existence. In a way, Kindelia can be seen as a decentralized
REPL that interprets statements in a programming language, with Kindelia blocks
being like files in such language, which are propagated and settled via proof of
work. It is as if a type theorist plugged a Python interpreter into a blockchain,
and ended up making a massively multiplayer [GHCi](https://downloads.haskell.org/~ghc/latest/docs/html/users_guide/ghci.html).

This is an extensive paper with a lot of information.
- If you'd like to see some example blocks, go to [Examples](#examples).
- If you'd rather read a long list of tech specs, head on to the [Technical Overview](#technical-overview).
- To learn about the optimal functional runtime we use, see [The High-order Virtual Machine](#the-high-order-virtual-machine-hvm).
- To learn how Kindelia compares to other p2p computers, move on to [Comparisons](#comparisons-to-ethereum).
- To learn the why's behind Kindelia's existence and design choices, go to [Whys](#whys).

Examples
========

Below are some example blocks. Keep in mind that the code shown, while
high-level looking, is just a **textual representation Kindelia's low-level
machine language**; i.e., it is equivalent to EVM's assembly, and **is not the
language developers are meant to use**. Instead, they should compile higher
level functional languages such as Haskell or, if they care about formal
verification, a proof language like [Kind](https://github.com/kindelia/kind).

Block #1: defining pure functions
---------------------------------

The **block** below defines and uses some global functions that operate on immutable trees:

```c
// Declares a constructor, Leaf, with 1 field
ctr {Leaf value}

// Declares a constructor, Branch, with 2 fields
ctr {Branch left right}

// Declares a pure function that sums a tree
fun (Sum tree) {
  (Sum {Leaf x})     = x
  (Sum {Branch a b}) = (+ (Sum a) (Sum b))
}

// Sums a tree with 4 numbers
run {
  !done (Sum {Branch
    {Branch {Leaf #1} {Leaf #2}}
    {Branch {Leaf #3} {Leaf #4}}})
}
```

You can run it offline by installing Kindelia and entering the command below:

```bash
# runs a kindelia block, displaying:
# - a list of each deployed ctr/fun
# - the result of each run{} block
# - total used mana and size
kindelia run block_1.kdl
```

When a Kindelia node runs that block, the global function `Sum` will be defined
forever inside the network. Note how it follows a functional style, closely
resembling Haskell's equational notation. Kindelia functions aren't compiled to
fit stack machines: they run natively as is, because beta reduction and pattern
matching are primitive, O(1) opcodes on the HVM.

Other than defining constructors and functions, blocks can also evaluate
side-effective actions inside `run {}` statements. These operate like Haskell's
`IO` monad. Inside them, users can query information from the blockchain, call
other functions, save and load a persistent state. Note that the `run{}`
statement shown on this example is not very useful, since it just performs a
pure computation and returns, without saving any state.

Kindelia's term language is lazy and linear, which means it works very similarly
to Haskell's runtime, except variables can't be used more than once, which is
essential to keep computations optimal and measurable. In order to make that
practical, there is a lazy duplication operation, written as `dup a b = x`,
which allows values to be cloned incrementally. Check
[HOW.md](https://github.com/Kindelia/HVM/blob/master/HOW.md) for more info on
how the HVM works.


Block #2: defining stateful functions
-------------------------------------

Actual contracts must hold a state. In Kindelia, every function holds an
internal state, which is just any native HVM structure. That state can be loaded
and saved using `load` and `save`. With just that, we are able to create
smart-contracts by using stateful functions.

Below, we define a `Count` contract that has two actions: one to increment a
counter, and one to return the current counter. We then run two IO blocks: one
that increments the counter 3 times, and other that just outputs the current
counter, i.e., 3.

```c
// Creates a Counter function with 2 actions:
ctr {Inc} // action that increments the counter
ctr {Get} // action that returns the counter
fun (Counter action) {

  // increments the counter
  (Counter {Inc}) =
    !take x        // loads the state and assigns it to 'x'
    !save (+ x #1) // overwrites the state as 'x + 1'
    !done #0       // returns 0

  // returns the counter
  (Counter {Get}) =
    !load x // loads the state
    !done x // returns it

// initial state is 0
} with {
  #0
}

// Increments the Counter's state 3 times
run {
  !call ~ 'Counter' [{Inc}]
  !call ~ 'Counter' [{Inc}]
  !call ~ 'Counter' [{Inc}]
  !done #0
}

// Prints the Counter's state
run {
  !call x 'Counter' [{Get}]
  !done x
}
```

Note that `load` and `save` aren't side-effective functions. Instead, they
*describe* effects using a pure datatype, exactly like Haskell's IO. These
effects can be passed as first-class expressions, and are evaluated when placed
directly inside a `run{}` block. For a complete list, see the section [IO
Effects](#io-effects) section.

A function's state can be any arbitrary HVM structure: a number, a list, a tree.
There is no forced, costly and error-prone `U256` serialization, like on
Ethereum. If a contract requires a balance map, for example, it can simply store
an immutable tree as its state, simply and directly.

Finally, `load` and `save` themselves have no cost! But blocks must still repeat
the heap growth and accumulated computation limites. So, for example, updating a
coin balance requires paying the cost of some `Map.updade` operation (which,
usually, is pretty cheap), but `save` itself has no cost, unlike `SSTORE`, which
is very expensive. This allows Kindelia to host highly dynamic applications such
as games and exchanges on its layer 1.

Block #3: signing statements
----------------------------

A `run{}` statement can also optionally include a signature:

```c
run {
  !name x // get the signer's name
  !done x // outputs it
} sign {
  0135ffd97b83f74843d93c4afb2d35d426c669f67bf3df8663de1d00768de179cd6215ccde6fe332845a5a4e72553bf9777b634b4f8ed91a1934620712e354887f
}
```

That hexadecimal string represents the secp256k1 signature of the serialization
of the `run{}` statement above. This will set the *subject* of the execution as
the signer, changing the behavior of the `IO.name` and `IO.from` primitives,
which return the subject's name, and the caller's name, respectively. To sign a
statement, just place it in the end of a `.kdl` file, and, enter the command:

```
kindelia sign block_file.kdl key_file
```

Block #4: playing a game
------------------------

```c
// TODO
```

Technical Overview
==================

Blocks and Statements
---------------------

Kindelia's network group user-submitted blocks using simple Nakamoto Consensus
(Proof of Work). Kindelia blocks are just groups of statements. Kindelia
statements can be one of 3 variants:

- **ctr**: declares a new constructor

    ```c
    ctr {ConstructorName field_0_name field_1_name ...}
    ```


- **fun**: declares a new function 

    ```c
    fun (FunctionName argument_0_name argument_1_name ...) {
      (ConstructorName arg_0 arg_1 ...) = returned_value_0
      (ConstructorName arg_0 arg_1 ...) = returned_value_1
      ...
    } with { initial_state }
    ```

- **run**: runs an IO expression

    ```c
    run {
      IO_expression
    }
    ```

Expressions
-----------

A Kindelia expression is a term in a pure, side-effect free, affine functional
language with 8 variants. Its grammar is described below:

```c
// A native number
Numb = Uint<120>

// A name
Name = Uint<60>

// A native int operation
Oper ::=
  +  // addition
  -  // subtraction
  *  // multiplication
  /  // division
  %  // modulus
  &  // bitwise and
  |  // bitwise or
  ^  // bitwise xor
  << // bitwise left shift
  >> // bitwise right shift
  <= // less than
  <  // less than or equal
  == // equal
  >  // greater than or equal
  >= // greater than
  != // not equal

// An expression
Term ::=

  // A lambda function
  @<var0: Name> <body: Term>
  
  // A lambda application
  (! <func: Term> <argm: Term>)
  
  // A constructor
  {<name: Name> <arg0: Term> <arg1: Term> ... <argN: Term>}
  
  // A function call
  (<name: Name> <arg0: Term> <arg1: Term> ... <argN: Term>)

  // A native number
  #<numb: Numb>

  // An integer operation
  (<oper: Oper> <val0: Term> <val1: Term>)

  // A cloning operation
  dup <var0: Name> <var1: Name> = <expr: Term>; <body: Term>

  // A variable
  <bind: Name>
```

For example,

```
@x @y {Pair (+ x #42) (F y)}
```

Denotes a function that receives two values, `x`, and `y`, and returns a pair
with `x` plus `42` and `F` applied to `y`. In Python, this could be written
as: `lambda x: lambda y: (x + 42, F(y))`. Also, since Kindelia's language is
affine, variables must occur, at most, once. To duplicate values, the clone
operator, `dup`, must be used.

While this syntax may look high-level, don't confuse it with a user-facing
language such as Solidity. It is just a direct textual representation of the
actual terms that run inside Kindelia's HVM. It should be seen as Kindelia's
low-level assembly, and developers should use higher-level languages (such as
Kind, Haskell, Idris, Agda, Coq and Lean) that compile to it.

Native numbers are 120-bit. The reason is that Kindelia's HVM uses 128-bit
pointers with a 8-bit tag. This allows storing numbers unboxed, as long as they
aren't larger than 120 bits. Any other size would less efficient: less than 120
bits would waste space, and more would require a pointer indirection. Numbers
are written with a `#`, followed by a decimal literal. For example, `#123456789`
is a valid number. As a syntax sugar, numbers can be also be written with single
quotes, containing a list of 6-bit letters, as follows:

      | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | A | B | C | D | E | F | 
    --|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
    0 | . | A | B | C | D | E | F | G | H | I | J | K | L | M | N | O | 
    1 | P | Q | R | S | T | U | V | W | X | Y | Z | a | b | c | d | e | 
    2 | f | g | h | i | j | k | l | m | n | o | p | q | r | s | t | u | 
    3 | v | w | x | y | z | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | _ | 

So, for example, `'Bar'` denotes the number `(0x02 << 12) | (0x1B << 6) | 0x2C`.
That naming convention can be used to give Kindelia-hosted applications
human-readable source codes.

IO Effects
----------

Finally, Kindelia has side-effective operations that allow functions to save
states, request information from the network. These operations are performed
using a pure description type that work similarly to Haskell's IO. In Haskellish
pseudocode, it would look like:

```
data IO s a
  = Done { retr :: a }                                             -- returns a value
  | Take { cont :: s -> (IO s a) }                                 -- takes the state
  | Save { expr :: s, cont :: () -> (IO s a) }                     -- saves the state
  | Call { func :: Name, args :: (ArgsOf func), cont :: (IO s a) } -- calls a function
  | Name { cont :: Name -> (IO s a) }                              -- gets the subject name
  | From { cont :: Name -> (IO s a) }                              -- gets the caller name
```


Note that, since Kindelia's language is pure, these side-effects are only
performed when placed directly inside top-level `run{}` statements, otherwise
they are treated as pure expressions, exactly like Haskell's IO. To receive
values from the external environment, a continuation (`@r ...`) is used. The
reason the code above isn't actual Haskell is that some of these constructors
would require dependent types to be expressed properly.

Note also that the `IO.take` operation actually removes the contract state,
rather than copying it. That's a consequence of HVM's linearity. This allows
implementing efficient functions that don't require cloning the state, but if
you don't include the state back later with `IO.save`, it will be emptied. As an
alternative, an `IO.load` function is defined on the genesis block, which works
exactly like `IO.take`, except it will clone the state.

Computation Rules
-----------------

Kindelia expressions are evaluated by the HVM, a functional virtual machine.
The primitive operations in that machine are called rewrite rules, and they
include beta reduction (lambda application), pattern-matching, numeric
operators, and primitives for cloning and erasing data. It is important to
stress that all these operations are constant-time, which is essential to make
costs measurable: see the mana table in the next section. For more info on how
that is possible, check HVM's [HOW.md](https://github.com/Kindelia/HVM/blob/master/HOW.md).

As explained on the document above, in addition to the 8 term variants, the HVM
also has an internal superposition construct, which is just a pair that can show
up as a byproduct of its lazy-cloning operation. That construct will be written
as `{a b}`. It also has an erasure construct, which may appear as a byproduct of
erasing data. Kindelia's rewrite rules are:

### Lambda Application

Applies a linear lambda to an argument. 

```
(@x body a)
----------- APP-LAM
x <- a
body
```

### Superposition Application

Applies a superposition to an argument.

```
({a b} c)
--------------- APP-SUP
dup x0 x1 = c
{(a x0) (b x1)}
```

### Lambda Duplication

Lazily, incrementally clones a lambda.

```
dup r s = @x f
-------------- DUP-LAM
dup f0 f1 = f
r <- @x0 f0
s <- @x1 f1
x <- {x0 x1}
```

### Superposition Duplication

Superpositions and duplications hold a 60-bit integer label. If the label is
equal, this rule collapses the superposition.

```
dup x y = {a b}
--------------- DUP-SUP (identical labels)
x <- a
y <- b
```

Otherwise, this rule duplicates the superposition.

```
dup x y = {a b}
--------------- DUP-SUP (different labels)
x <- {xA xB}
y <- {yA yB}
dup xA yA = a
dup xB yB = b
```

### Number Duplication

Clones a 120-bit number.

```
dup x y = N
----------- DUP-NUM
x <- N
y <- N
```

### Constructor Duplication

Lazily, incrementally clones a constructor.

```
dup x y = (K a b c ...)
----------------------- DUP-CTR
dup a0 a1 = a
dup b0 b1 = b
dup c0 c1 = c
...
x <- (K a0 b0 c0 ...)
y <- (K a1 b1 c1 ...)
```

### Erasure Duplication

Erases a duplication.

```
dup x y = ~
----------- DUP-ERA
x <- ~
y <- ~
```

### Numeric Operation

A binary operation on 120-bit numbers.

```
(+ a b)
--------- OP2-NUM
add(a, b)
```

The numeric operations available are:

ID | Name      | Symbol | Operation
-- | --------- | ------ | --------------------------------------
 0 | ADD       | `+`    | unsigned integer addition
 1 | SUB       | `-`    | unsigned integer subtraction
 2 | MUL       | `*`    | unsigned integer multiplication
 3 | DIV       | `/`    | unsigned integer division
 4 | MOD       | `%`    | unsigned integer modulus
 5 | AND       | `&`    | unsigned integer bitwise and
 6 | OR        | `\|`   | unsigned integer bitwise or
 7 | XOR       | `^`    | unsigned integer bitwise xor
 8 | SHL       | `<<`   | unsigned integer bitwise left shift
 9 | SHR       | `>>`   | unsigned integer bitwise right shift
10 | LE        | `<=`   | unsigned integer less than or equal
11 | LT        | `<`    | unsigned integer less than
12 | EQ        | `==`   | unsigned integer equal
13 | GT        | `>`    | unsigned integer greater than or equal
14 | GE        | `>=`   | unsigned integer greater than
15 | NE        | `!=`   | unsigned integer not equal

Numeric operations have type `fn(u120,u120) -> u120`. Comparison operations like
`equal` return 0 for false and 1 for true.

### Superposed Operation

A binary operation on a superposition.

```
(+ {a0 a1} b)
--------------------- OP2-SUP-0
let b0 b1 = b
{(+ a0 b0) (+ a1 b1)}
```

```
(+ a {b0 b1})
--------------------- OP2-SUP-1
dup a0 a1 = a
{(+ a0 b0) (+ a1 b1)}
```

### Superposed Matching

Pattern-matching on a superposition.

```
(F {a0 a1} b c ...)
----------------------------------- FUN-SUP
dup b0 b1 = b
dup c0 c1 = c
...
{(F a0 b0 c0 ...) (F a1 b1 c1 ...)}
```

### Constructor Matching

Pattern-matching on a constructor. Each user-defined equation installs a new,
global pattern-matching rewrite rule.

```
(user-defined)
-------------- FUN-CTR
(user-defined)
```

Memory Model
------------

HVM's memory model is documented on `src/hvm.rs`, and trascribed below:

```
HVM's runtime memory consists of a vector of u128 pointers. That is:

  Mem ::= Vec<Ptr>

A pointer has 3 parts:

  Ptr ::= TT AAAAAAAAAAAAAAA BBBBBBBBBBBBBBB

Where:

  T : u8  is the pointer tag 
  A : u60 is the 1st value
  B : u60 is the 2nd value

There are 12 possible tags:

  Tag | Val | Meaning  
  ----| --- | -------------------------------
  DP0 |   0 | a variable, bound to the 1st argument of a duplication
  DP1 |   1 | a variable, bound to the 2nd argument of a duplication
  VAR |   2 | a variable, bound to the one argument of a lambda
  ARG |   3 | an used argument of a lambda or duplication
  ERA |   4 | an erased argument of a lambda or duplication
  LAM |   5 | a lambda
  APP |   6 | an application
  SUP |   7 | a superposition
  CTR |   8 | a constructor
  FUN |   9 | a function
  OP2 |  10 | a numeric operation
  NUM |  11 | a 120-bit number

The semantics of the 1st and 2nd values depend on the pointer tag. 

  Tag | 1st ptr value                | 2nd ptr value
  --- | ---------------------------- | ---------------------------------
  DP0 | the duplication label        | points to the duplication node
  DP1 | the duplication label        | points to the duplication node
  VAR | not used                     | points to the lambda node
  ARG | not used                     | points to the variable occurrence
  ERA | not used                     | not used
  LAM | not used                     | points to the lambda node
  APP | not used                     | points to the application node
  SUP | the duplication label        | points to the superposition node
  CTR | the constructor name         | points to the constructor node
  FUN | the function name            | points to the function node
  OP2 | the operation name           | points to the operation node
  NUM | the most significant 60 bits | the least significant 60 bits

Notes:

  1. The duplication label is an internal value used on the DUP-SUP rule.
  2. The operation name only uses 4 of the 60 bits, as there are only 16 ops.
  3. NUM pointers don't point anywhere, they just store the number directly.

A node is a tuple of N pointers stored on sequential memory indices.
The meaning of each index depends on the node. There are 7 types:

  Duplication Node:
  - [0] => either an ERA or an ARG pointing to the 1st variable location
  - [1] => either an ERA or an ARG pointing to the 2nd variable location
  - [2] => pointer to the duplicated expression

  Lambda Node:
  - [0] => either and ERA or an ERA pointing to the variable location
  - [1] => pointer to the lambda's body
  
  Application Node:
  - [0] => pointer to the lambda
  - [1] => pointer to the argument

  Superposition Node:
  - [0] => pointer to the 1st superposed value
  - [1] => pointer to the 2sd superposed value

  Constructor Node:
  - [0] => pointer to the 1st field
  - [1] => pointer to the 2nd field
  - ... => ...
  - [N] => pointer to the Nth field

  Function Node:
  - [0] => pointer to the 1st argument
  - [1] => pointer to the 2nd argument
  - ... => ...
  - [N] => pointer to the Nth argument

  Operation Node:
  - [0] => pointer to the 1st operand
  - [1] => pointer to the 2nd operand

Notes:

  1. Duplication nodes DON'T have a body. They "float" on the global scope.
  2. Lambdas and Duplications point to their variables, and vice-versa.
  3. ARG pointers can only show up inside Lambdas and Duplications.
  4. Nums and vars don't require a node type, because they're unboxed.
  5. Function and Constructor arities depends on the user-provided definition.

Example 0:

  Term:

   {Tuple2 #7 #8}

  Memory:

    Root : Ptr(CTR, 0x0000007b9d30a43, 0x000000000000000)
    0x00 | Ptr(NUM, 0x000000000000000, 0x000000000000007) // the tuple's 1st field
    0x01 | Ptr(NUM, 0x000000000000000, 0x000000000000008) // the tuple's 2nd field

  Notes:
    
    1. This is just a pair with two numbers.
    2. The root pointer is not stored on memory.
    3. The '0x0000007b9d30a43' constant encodes the 'Tuple2' name.
    4. Since nums are unboxed, a 2-tuple uses 2 memory slots, or 32 bytes.

Example 1:

  Term:

    λ~ λb b

  Memory:

    Root : Ptr(LAM, 0x000000000000000, 0x000000000000000)
    0x00 | Ptr(ERA, 0x000000000000000, 0x000000000000000) // 1st lambda's argument
    0x01 | Ptr(LAM, 0x000000000000000, 0x000000000000002) // 1st lambda's body
    0x02 | Ptr(ARG, 0x000000000000000, 0x000000000000003) // 2nd lambda's argument
    0x03 | Ptr(VAR, 0x000000000000000, 0x000000000000002) // 2nd lambda's body

  Notes:

    1. This is a λ-term that discards the 1st argument and returns the 2nd.
    2. The 1st lambda's argument not used, thus, an ERA pointer.
    3. The 2nd lambda's argument points to its variable, and vice-versa.
    4. Each lambda uses 2 memory slots. This term uses 64 bytes in total.
    
Example 2:

  Term:
    
    λx dup x0 x1 = x; (* x0 x1)

  Memory:

    Root : Ptr(LAM, 0x000000000000000, 0x000000000000000)
    0x00 | Ptr(ARG, 0x000000000000000, 0x000000000000004) // the lambda's argument
    0x01 | Ptr(OP2, 0x000000000000002, 0x000000000000005) // the lambda's body
    0x02 | Ptr(ARG, 0x000000000000000, 0x000000000000005) // the duplication's 1st argument
    0x03 | Ptr(ARG, 0x000000000000000, 0x000000000000006) // the duplication's 2nd argument
    0x04 | Ptr(VAR, 0x000000000000000, 0x000000000000000) // the duplicated expression
    0x05 | Ptr(DP0, 0x3e8d2b9ba31fb21, 0x000000000000002) // the operator's 1st operand
    0x06 | Ptr(DP1, 0x3e8d2b9ba31fb21, 0x000000000000002) // the operator's 2st operand

  Notes:
    
    1. This is a lambda function that squares a number.
    2. Notice how every ARGs point to a VAR/DP0/DP1, that points back its source node.
    3. DP1 does not point to its ARG. It points to the duplication node, which is at 0x02.
    4. The lambda's body does not point to the dup node, but to the operator. Dup nodes float.
    5. 0x3e8d2b9ba31fb21 is a globally unique random label assigned to the duplication node.
    6. That duplication label is stored on the DP0/DP1 that point to the node, not on the node.
    7. A lambda uses 2 memory slots, a duplication uses 3, an operator uses 2. Total: 112 bytes.
    8. In-memory size is different to, and larger than, serialization size.
```

HVM's runtime is essentially a lazy graph traversal machine that finds redexes
(expressions subject to computation rules) and rewrites them until there is no
more work to do. It does so while automatically allocating and freeing memory of
expressions that go out of scope. When a term's reduction is complete, HVM's
memory will be fully emptied, leaving no leaks, which is what allows Kindelia to
replace state trees by heap snapshots. The only exception is when an app
explicitly asks to preserve an expression by using the `IO.save` operation,
which will simply store a pointer to the app's state, and keep it in memory.
That is why Kindelia's "SSTORE" has zero-cost: the operation itself is very
cheap as it just saves a pointer. We only have to "charge" an app when it uses
more of the available heap space.

Genesis Block
-------------

Kindelia starts the network by running a single block before the first mined
block. This is called the genesis block. That block installs some utilities on
the network, as shown below:

```c
// Tuple types
ctr {Tuple0}
ctr {Tuple1 x0}
ctr {Tuple2 x0 x1}
ctr {Tuple3 x0 x1 x2}
ctr {Tuple4 x0 x1 x2 x3}
ctr {Tuple5 x0 x1 x2 x3 x4}
ctr {Tuple6 x0 x1 x2 x3 x4 x5}
ctr {Tuple7 x0 x1 x2 x3 x4 x5 x6}
ctr {Tuple8 x0 x1 x2 x3 x4 x5 x6 x7}
ctr {Tuple9 x0 x1 x2 x3 x4 x5 x6 x7 x8}
ctr {Tuple10 x0 x1 x2 x3 x4 x5 x6 x7 x8 x9}
ctr {Tuple11 x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10}
ctr {Tuple12 x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11}

// Used to pretty-print names
ctr {Name name}

// Below, we declare the built-in IO operations

// IO.done returns from an IO operation
ctr {IO.DONE expr}
fun (IO.done expr) {
  (IO.done expr) = {IO.DONE expr}
}

// IO.take recovers an app's stored state
ctr {IO.TAKE then}
fun (IO.take then) {
  (IO.take then) = {IO.TAKE then}
}

// IO.save stores the app's state
ctr {IO.SAVE expr then}
fun (IO.save expr then) {
  (IO.save expr then) = {IO.SAVE expr then}
}

// IO.call calls another IO operation, assigning
// the caller name to the current subject name
ctr {IO.CALL name args then}
fun (IO.call name args then) {
  (IO.call name args then) = {IO.CALL name args then}
}

// IO.name returns the name of the current subject
ctr {IO.NAME then}
fun (IO.name then) {
  (IO.name then) = {IO.NAME then}
}

// IO.from returns the name of the current caller
ctr {IO.FROM then} 
fun (IO.from then) {
  (IO.from then) = {IO.FROM then}
}

// Works like IO.take, but clones the state
fun (IO.load cont) {
  (IO.load cont) =
    {IO.TAKE @x
    dup x0 x1 = x;
    {IO.SAVE x0 @~
    (! cont x1)}}
}
```

Table of Costs
--------------

Since Kindelia's built-in language is Turing complete, it must have a way to
account for, and limit, performed computations; otherwise, anyone could freeze
the entire network by deploying infinite loops, or expensive computations. Like
Ethereum, it has a cost table linking primitive operations to a number, which is
called mana instead of gas. Unlike Ethereum, that cost isn't associated with
transactions, but with the block as a whole.

    .---------------------------------------------------.
    | Opcode  | Effect                          | Mana  |
    |---------|---------------------------------|-------|
    | APP-LAM | applies a lambda                | 2     |
    | APP-SUP | applies a superposition         | 4     |
    | OP2-NUM | operates on a number            | 2     |
    | OP2-SUP | operates on a superposition     | 4     |
    | FUN-CTR | pattern-matches a constructor   | 2 + M |
    | FUN-SUP | pattern-matches a superposition | 2 + A |
    | DUP-LAM | clones a lambda                 | 4     |
    | DUP-NUM | clones a number                 | 2     |
    | DUP-CTR | clones a constructor            | 2 + A |
    | DUP-SUP | clones a superposition          | 4     |
    | DUP-SUP | undoes a superposition          | 2     |
    | DUP-ERA | clones an erasure               | 2     |
    |---------------------------------------------------|
    | * A is the constructor or function arity          |
    | * M is the alloc count of the right-hand side     |
    '---------------------------------------------------'


Kindelia's elegant runtime is reflected by the simplicity of this table. In
order to limit computations, nodes impose a hard ceiling on the amount of
computation performed, as a function of the block number:

```
mana_limit = 10000000 * (block_number + 1)
```

If a block passes that limit, it is rejected by nodes. Note that this limit
accumulates: if a block doesn't fully use it, the next block can use it, and so
on. In effect, that causes times of low usage to "lend" computation to times of
high usage, making Kindelia somewhat resistant to performance losses due to
high-traffic applications or periods, while still keeping the maximum
synchronization computation in check.

The current Rust implementation is capable of computing about 55 million mana
per second in an Apple M1 processor. This is about 7 times larger than the mana
limit per block. That means that, for every 7 seconds a node spends offline, it
must spend 1 second catching up, if single threaded. While that isn't a huge
margin, blocks could be processed in a parallel fashion, and future improvements
on the HVM and processors will improve this margin.

Kindelia also has a hard ceiling on the state size, i.e., the size of its heap:

```
bits_limit = 2048 * (block_number + 1)
```

That means that, for every second that passes, the state size is allowed to grow
2048 bits. That is equivalent to an HVM constructor with 16 numeric fields, or 8
HVM lambdas. That amounts to a blockchain state growth of about 8 GB per year.
Just like mana, this accumulates, so, for example, if there are 3 empty blocks,
the 4th block will be able to let the blockchain size grow up to 8192 bits.

### About miner fees

An attentive reader may have noticed that there is no miner fee mechanism
included on this implementation. That is by design. Kindelia restricts how much
computation and space the network may use in total as a function of its age, but
it says nothing about individual transactions. Kindelia relies on the principle
that, during the early ages of the network, users will be mining their own
blocks directly. After all, with 1 second per block, there are 86400 blocks per
day. Until there are thousands of active users, mining a block won't be an issue
for an average user. When that starts becoming practical, a fee market will
emerge naturally, and users will pay miners in whatever currencies they like.
For more on that, check the ["Why not include a currency?"](#why-not-include-a-currency) section.

Serialization
-------------

Kindelia blocks are serialized to binary following the procedures below:

### Fixlen

The Fixlen encoding serializes unsigned integers of known length by their binary
representation, except reversed.

```
serialize_fixlen(s, n) = right_pad(0, s, n.to_binary())
```

It is reversed in order to represent the least significant bit first, making it
consistent with the varlen encoding. Example:

```
serialize_fixlen(8, 19) = 11001000
```

That's because `19` in binary is `10011`. Reversing, we get `11001`. Padding 3
zeroes right, we get `11001000`.

### Varlen

The Varlen encoding serializes unsigned integers of unknown length as a list of
bits containing the reversed binary representation of the number.

```
serialize_varlen(n) = serialize_list(n.to_binary())
```

Since the size is unknown, the bit `1` means there is a bit to read, and the bit
`0` means the sequence has ended. Example:

```
fixlen(5,19) =  1  0  0  0  1
varlen(  19) = 11 10 10 10 11 0
```

The number `19` is represented as `11101010110`, which is just the
`fixlen(5,19)` representation with `1`'s before each significant bit, and `0` at
the end. In this case, `varlen` uses `6` bits more than `fixlen`, which is the
cost of not knowning the size statically.

### List

The List encoding serializes a list of unknown length, by using `1` to introduce
a new list element, and `0` to denote the end of the list. It is parametric on
the function that serializes a single element, `serialize_elem`.

```
serialize_list(serialize_elem, cons(x,xs)) = 1 | serialize_elem(x) | serialize_list(xs)
serialize_list(serialize_elem, nil)        = 0
```

Note that this format is different than the usual encoding of sequences, where
the size is encoded, followed by a series of serialized values. It is more
efficient when the list length is smaller than 64, which is the typical case in
every instance where `serialize_list` is used.

### Number

The Number encoding serializes unsigned integers in a compressed form.

```
serialize_number(n) = varlen(bit_size(n)) | fixlen(n)
```

It is used to serialize numbers of unknown length, and is more efficient than
`serialize_varlen` when the number is larger than 63. For example:

```
bit_size(1337) = 11
serialize_number(1337) = 11 11 10 11 0 | 10011100101
                         '-----------'   '---------'
                          varlen(11)      fixlen(11,1337)
```

Here, encoding `1337` requires 20 bits, which is 3 bits less than varlen would
use, and 100 bits less than encoding all the 120 bits would require.

### Name

The Name encoding serializes names as lists of 6-bit letters.

```
serialize_name(name) = 0 | serialize_list(λ x => serialize_fixlen(6,x), name)
```

It uses the letter table shown earlier to represent names tersely.

```
serialize_name('Dog') = 0 1 110101 1 110011 1 011100 0
                            '----'   '----'   '----'
                             'D'      'o'      'g'
```

Since all names are smaller than 64 characters, using `serialize_list` is more
efficient than encoding the length of the name, followed by the character.

Note this encoding starts with a `0` bit that isn't used yet. That is a
compressed-name flag. Names are the most data-hungry part of Kindelia's
serialization, yet, there are many instances where names can be compressed
considerably. For example, variable names can be compressed using De Bruijn
indices, and constructor/function names can be compressed by local name aliases
when these are used repeatedly. These optimizations aren't implemented yet, but,
in order to make that possible in a future, we reserve a bit for this flag.

Note also that, while the serialization of a name allows for an arbitrary number
of letters, HVM constructors and functions reserve 60 bits for the name, which
means the maximum constructor name is 10 letters long, and that it is impossible
to call a function with a name larger than 10 letters directly. Functions with
11-20 letters are deployable, but they can only be called with `IO.call`, which
receives a 120-bit number.

### Term

The Term encoding serializes an HVM term, or expression. It uses a 3-bit tag to
represent the term variant, followed by the serialization of each field.

```
serialize_term(Var(name))
  = serialize_fixlen(3,0)
  + serialize_name(name)

serialize_term(Dup(nam0,nam1,expr,body))
  = serialize_fixlen(3,1)
  + serialize_name(nam0)
  + serialize_name(nam1)
  + serialize_term(expr)
  + serialize_term(body)

serialize_term(Lam(name,body))
  = serialize_fixlen(3,2)
  + serialize_name(name)
  + serialize_term(body)

serialize_term(App(func,argm))
  = serialize_fixlen(3,3)
  + serialize_term(func)
  + serialize_term(argm)

serialize_term(Ctr(name,args))
  = serialize_fixlen(3,4)
  + serialize_name(name)
  + serialize_list(serialize_term, args)

serialize_term(Fun(name,args))
  = serialize_fixlen(3,5)
  + serialize_name(name)
  + serialize_list(serialize_term, args)

serialize_term(Num(numb))
  = serialize_fixlen(3,6)
  + serialize_number(numb)

serialize_term(Op2(oper, val0, val1))
  = serialize_fixlen(3,7)
  + serialize_fixlen(4, oper)
  + serialize_term(val0)
  + serialize_term(val1)
```

Note that constructors and function calls can't have more than 15 fields or
arguments, thus, using `serialize_list` is optimal here too. Note also how
`serialize_number` is used to serialize numeric constants, which is typically
more efficient than `serialize_varlen` and `serialize_fixlen` would be. Finally,
numeric operations are serialized using 4 bits, which is enough to store the 16
primitives that the HVM has.

Since constructor tags only use 3 bits, this allows for compact serialization of
expressions and functions. For example:

```
                                Lam 'x'       Lam 'y'       Op2 Add  Var 'x'       Var 'y'
serialize_term(@x @y (+ x y)) = 010 010011110 010 011011110 111 0000 000 010011110 000 011011110
```

This is an anonymous function that adds two numbers. It only uses 55 bits, or
less than 7 bytes. Notice, though, how names use most of the space. In a future
update, compressed names will shorten that to 27 bits, or about 3 bytes:

```
                                Lam   Lam   Op2 Add  Var 1    Var 0
serialize_term(@x @y (+ x y)) = 010 1 010 1 111 0000 000 1101 000 10
```

Here, the compressed-name flag is used to both make anonymous functions with no
variable name, and to let variables address their binding lambdas using De
Bruijn indices.

### Statement

The Statement encoding serializes a top-level statement in a Kindelia block.

```
serialize_rule((lhs,rhs))
  = serialize_term(lhs)
  + serialize_term(rhs)

serialize_statement(Fun(name,args,func,init))
  = serialize_fixlen(4, 0)
  + serialize_name(name)
  + serialize_list(serialize_name, args)
  + serialize_list(serialize_rule, func)
  + serialize_term(init)

serialize_statement(Ctr(name,ctrs))
  = serialize_fixlen(4, 1)
  + serialize_name(name)
  + serialize_list(serialize_name, ctrs)

serialize_statement(Run(expr))
  = serialize_fixlen(4, 2)
  + serialize_term(expr)
```


### Block

The Block encoding serializes a list of top-level statements, i.e., a block.

```
serialize_block(statements) = serialize_list(serialize_statement, statements)
```

The High-order Virtual Machine (HVM)
====================================

As stated, one of the most profound differences between Kindelia and other
computation networks is the High-order Virtual Machine (HVM). It is a massively
parallel runtime capable of evaluating functional programs optimally, a
remarkable property that no other compiler in the market enjoys. Even though it
is still in its infancy, it already compares to Haskell's GHC, the most mature
pure functional compiler in the world, in most practical benchmarks.

### 1. For normal, sequential tasks, it shows similar performance:

![sequential](https://github.com/Kindelia/HVM/raw/master/bench/_results_/ListFold.png)

### 2. For very parallel tasks, it is several times faster:

![parallel](https://github.com/Kindelia/HVM/raw/master/bench/_results_/QuickSort.png)

### 3. For very high-order tasks, it is exponentially faster:

![high-order](https://github.com/Kindelia/HVM/raw/master/bench/_results_/LambdaArithmetic.png)

The charts speak for themselves: HVM, on its first prototype, competes with
Haskell's 30-years-old compiler. On the long term, it can bring unforeseen
improvements on the performance of functional programs, specially taking in
account the rise of parallel processors, and the discovery of new data
structures and algorithms that take advantage of beta-optimality.

When it comes to Kindelia, though, its main role is to make functional programs
and algorithms run natively, which directly reduces costs by 100x to 1000x,
compared to emulating them on stack machines such as the EVM. Other than raw
performance, there are two additional characteristics that make it the ideal
virtual machine for a functional peer-to-peer computer:

### 1. It has negligible compilation times.

Peer-to-peer computers must evaluate programs as fast as possible to increase
throughput, yet, they can not use traditional optimizing compilers such as GCC
or GHC. That's because an attacker could forge programs that take a long time to
compile, resulting in a denial-of-service (DoS) attack that would stop the
entire network. That is why Ethereum can't compile contracts, and why Cardano
can't use the GHC, even though its contracts are written in Haskell.

The HVM achieves its performance without a slow compiler. Instead, a single pass
converts serialized programs coming from blocks straight into runtime objects on
the node's memory. That pass is so cheap that it doesn't even need to be
accounted for, making the HVM, unlike GHC, capable of dealing with arbitrary
user-submitted code without exposing DoS vectors.

### 2. Computation costs are measurable, including space and time.

Even if GHC had a fast, single-pass compiler, using its runtime as the backbone
of a peer-to-peer decentralized computer wouldn't be viable, because some
operations such as lambda application (also called beta reduction) can have
unpredictable costs, depending on the context. It would be very hard to come up
with a cost table that reflected real-world timings. To complicate matters
further, Haskell's memory model would cause the heap to grow every time a
contract is called, until its global garbage collector was triggered,
potentially freezing the network, and raising a hard question: who pays for it,
the user that triggered the GC, or the app that leaked the memory?

HVM has the best of all worlds. Due to linearity, its beta reduction opcode is
an O(1), lightweight operation that can be assigned a fixed cost, exactly like
any arithmetic opcode on Ethereum; see the [Table of Costs](#table-of-costs)
below. Moreover, it is garbage-collection free: a transaction always clears all
the (non-store-persisted) memory it uses as soon as it completes. There are no
leaks and no stop-the-world garbage collectors.  All that while maintaining the
lazy functional semantics that allow pure ultra-secure, pure functional
languages to run natively.

### How is that possible?

For a more in-depth overview, check the [HVM](https://github.com/kindelia/hvm)
repository and, in particular,
[HOW.md](https://github.com/Kindelia/HVM/blob/master/HOW.md). The solution is
simpler than it looks, and boils down to a combination of linearity, with a lazy
duplication primitive that incrementally copies any term, including lambdas.
Behind this simple concept, lies an elegant model of computation, the
Interaction Net, which shares the best aspects of the Turing Machines and the
Lambda Calculus, in a manner that looks truly foundamental, and is the main
reason the HVM works so well.

### Is Kindelia a consequence of the HVM?

Yes. The main motivation behind the creation of Kindelia is to spread awareness
of the HVM, and all the amazing possibilities that Interaction Nets bring - and
we're very open about that. Once the world realizes the potential of the HVM
through Kindelia, it won't be hard to foresee a future where massively parallel
interaction-net processors will replace the old Von Neumann architecture, taking
humanity to a whole new level of technological maturity. As the authors of
Kindelia, we intend to lead this revolution.

This, by itself, is a fair reason to believe on Kindelia's future; after all, we
are, ironically, doing this for the technology, and have no interest on
profiting from the network. That is why we made an extra effort to make it as
simple and stable as possible, decreasing the need for a highly active core
team, and, thus, removing our own roles as figureheads. We further reinforce
that philosophy by not adding a native currency that is massively pre-mined by
the creators, as most projects do.

Note we do intend to launch a token for our foundation as a contract *inside*
Kindelia, but it will not be *part* of the network, nor coupled to Kindelia's
protocol in any way, as things should be. Since Kindelia shouldn't require
high maintainance costs, that asset that will be used mostly to fund the
development of the HVM, and its next-gen parallel compilers and processors.

Comparisons to Ethereum
=======================

### HVM makes functional and formally verified DApps much cheaper

Formal verification can be used to mathematically ensure that a program can't be
exploited, which is invaluable for a network where programs can't be patched or
reversed, but Ethereum's virtual machine is too inefficient for functional
programs [citation], making it unpractical: devs must either verify the
generated bytecode, which is extremelly hard and laborious, or compile a proof
language such as Idris, which results in expensive EVM contracts. Thanks to the
HVM, Kindelia is able to perform beta reduction and pattern-matching natively,
making functional programs much cheaper, which, in turn, makes formally verified
contracts economically viable.

### Reversible heaps make dynamic DApps much cheaper

One of the most expensive operations on Ethereum is SSTORE, which saves a U256
number permanently. It costs 20000 gas for a new write, and 5000 gas for a
rewrite. That cost is high for two reasons: first, to limit the state growth;
second, because it uses expensive Merkle Patricia Tree insertions. Kindelia
treats state completely differently: it just saves reversible snapshots of HVM's
heap! Because of that, a reused SSTORE has 0 cost. This makes programming much
more convenient, since contracts can store arbitrary HVM structures like trees
and JSONs instead of just U256s; but, more importantly, it makes highly dynamic
layer-1 DApps significantly cheaper.

### Optimizations and simplifications everywhere

Kindelia's block structure is refreshingly simple. There are no merkle roots,
bloom filters, logs, receipts. Just a timestamp, the previous hash, the miner
id, and a list of "statements" that alter the network's state, in a way that
resembles a p2p REPL. The entire block is less than *1500 bytes* long, and fits
in a single UDP packet, allowing fast fast propagation and short block times.
There are no monetary transactions, just statements, which can be compressed and
unsigned, greatly reducing the size and cost of several types of transactions,
including app deployment. Contracts aren't compiled to monolithic assembly
codes, but are, instead, broken into pure functions that are deployed separately
and modularly, enabling massive code reuse. Finally, there is no native
currency, making Kindelia not a cryptocurrency, and allowing users to pay miners
in any on-chain asset, rather than the network's "built-in token".

### Comparison table

The table below compares some attributes of each network:

.                               |                                  Kindelia |                              Ethereum
------------------------------- | ----------------------------------------: | ------------------------------------:
**block time**                  | ```                         1 second  ``` | ```                       13 seconds  ```
**block size**                  | ```                       1280 bytes  ``` | ```                             1 MB  ```
**max throughput: signed tx**   | ```                         ~20 tx/s  ``` | ```                         ~30 tx/s  ```
**max throughput: unsigned tx** | ```                        ~160 tx/s  ``` | ```                              N/A  ```
**max growth: blockchain**      | ```                       40 GB/year  ``` | ```                    2,174 GB/year  ```
**max growth: state heap**      | ```  56 byte/s ~           8 GB/year  ``` | ```   3,680 byte/s ~     114 GB/year  ```
**max growth: computation**     | ```                10,000,000 mana/s  ``` | ```                  2,300,000 gas/s  ```
**cost: multiplication**        | ```     2 mana ~      5,000,000 op/s  ``` | ```          5 gas ~    460,000 op/s  ```
**cost: beta reduction**        | ```     2 mana ~      5,000,000 op/s  ``` | ```       ~200 gas ~     11,500 op/s  ```
**cost: pattern matching**      | ```     2 mana ~      5,000,000 op/s  ``` | ```       ~200 gas ~     11,500 op/s  ```
**cost: SSTORE (reuse)**        | ```     0 bits ~      5,000,000 op/s  ``` | ```      5,000 gas ~        460 op/s  ```
**cost: SSTORE (alloc)**        | ```   128 bits ~             16 op/s  ``` | ```     20,000 gas ~        115 op/s  ```

The costs in this table were defined based on HVM benchmarks, using modern
mid-end processors.

Network-wise, Kindelia's block time is shorter, because its compressed blocks
fit in a single UDP packet. Due to block size limits, Ethereum can handle
slightly more signed transactions per second, including monetary transfers, but
Kindelia handles considerably more unsigned transactions per second, including
contract deployment and other interactions that don't require authentication.
Note that, since Kindelia's signed transactions can group multiple calls in a
single statement, the actual throughput can be much higher, and it achieves that
with a fraction of Ethereum's maximum blockchain growth.

Computation-wise, Kindelia's layer-1 throughput is up to 434x higher, due to the
HVM and stateful heaps respectivelly. Kindelia's functional opcodes allow it to
host programs compiled from secure languages like Haskell, Idris, Agda and Kind,
which is simply not economically viable on Ethereum. Kindelia's zero-cost reused
SSTORE enables highly dynamic applications like layer-1 MMORPGs, which is also
not viable on Ethereum. See the complete [Table of Costs](#table-of-costs)
below.

Space-wise, Kindelia has more strict limits on the blockchain and state heap
growth, intentionally capping the maximum growth, to keep it in check.
Ethereum's blockchain could, in theory, grow 2 terabytes per year, and its state
could grow 114 gigabytes per year, which would increase centralization, and make
it difficult to store the state in-memory. Note that, on Ethereum, this
theoretical limit isn't meant to be reached, while Kindelia is designed to
always grow at its max rate, so these numbers aren't directly comparable.

### In short

Kindelia is just a layer-1 decentralized computer that resembles Ethereum,
except without a native currency, with a VM that makes functional programs much
cheaper, and with a state manager that makes dynamic apps much cheaper. 

Finally, note that we do not claim that Kindelia is *better* than Ethereum. It
is just different, with different goals. Kindelia is minimalist, Ethereum is
complex. Ethereum has features that some may miss on Kindelia. For example,
Ethereum's Merkle Patricia Trees are terrible for SSTORE performance, but they
allow light clients, which Kindelia lacks. Kindelia doesn't store logs, bloom
filters, there is no GHOST protocol, and consensus is just old and simple proof
of work, rather than complex proof of stake schemes. Whether that's better or
worse, it depends on the use case.

Comparisons to Cardano
======================

Cardano uses Haskell as its scripting language, which, when it comes to security
and efficiency, seems like a great choice, because Haskell is functional, and
its compiler, the GHC, is extremelly efficient. In a closer inspection, though,
that approach doesn't make so much sense.

### On security

When it comes to **security**, while Haskell is functional, it does not actually
feature formal proofs natively. That means users can not write a contract and
prove theorems in the same language: they need to compile them to *something
else*, and then prove theorems about the *compiled output*. This is much more
expensive than developing and proving in the same language, and isn't much
better than Ethereum's situation. In Kindelia, users write and verify contracts
in Kind, Idris or other proof assistant, and run them directly on the HVM. It is
simple, cheap, secure and works.

### On efficiency

When it comes to **efficiency**, while Haskell's compiler, the GHC, produces
efficient programs, it is not actually used to run contracts on Cardano!  That's
because GHC's runtime wasn't designed with a peer-to-peer setup in mind, so,
using it would raise complications, such as high compilation times, and how to
measure execution costs. Because of that, contracts are compiled to an
intermediate language, Plutus, and interpreted on-chain. This is much less
efficient than compiling, and ultimately means that Cardano fees can't be much
cheaper than Ethereum's, under the same load. Kindelia's HVM is as efficient as
GHC, while having negligible compilation times, and measurable execution costs,
letting users run Haskell-like languages with GHC-like efficiency.

### On simplicity

Finally, Cardano's extended UTXO model, derived from Bitcoin, makes it
currency-first, computer-second. It greatly increases network complexity and
decreases contract expressivity, under the assumption that it improves
efficiency, which is dubious at least, since, untimately, efficiency is still
dictated by total memory, disk and computation usage. The way programs are
triggered is irrelevant for that analysis. These is no clarity on how much of
these resources Cardano would use under high load, and what its opcode costs and
limits are. Kindelia has no UTXO; it is just a functional runtime, with a global
state, run/deploy transactions that alter it, and clear costs and limits for
every defined operation.

### In short

In short, Cardano's goals are aligned to ours, but the execution differs
drastically. Instead of fancy ideas that sound more like academic show-off than
practical utilities, Kindelia is just the result of wrapping a functional
runtime, the HVM, in a currency-less blockchain, which is just the right recipe
to make the ultra-secure, worldwide functional computer that Cardano envisioned,
but couldn't deliver. Finally, instead of a massive, VC-funded company building
a highly complex system, Kindelia is just a minimal open-source standard - our
entire implementation is under 10k lines of code! - promoting node diversity and
making it inherently independent from its original creators.

Finally, note that we do not claim that Kindelia is *better* than Cardano. It is
impossible to innovate without trying new things, and it is okay if some ideas
don't work so well. Cardano has great minds, and has everything it takes to
build a great decentralized computer.


Whys
====

## Why create a cryptocurrency?

*“Your scientists were so preoccupied with whether they could, they didn't stop to think if they should.”*

This is a hard question to answer because, well, Kindelia is **not** a
cryptocurrency. Yes, it uses a blockchain, and yes, you can create currencies on
it, but you can also create currencies on your computer, and you don't call it
an "electrocurrency". Kindelia is less like a currency a more like an operating
system, except it is virtual and synchronized worldwide. I get the question,
though. It often comes from people frustrated with the state of the field, ICOs,
NFTs, pyramids, scams and idiocratic claims everywhere. It is true that most
blockchain projects are a solution looking for a problem. But so could be said
about early internet companies. There is still some value to be found in them,
because they solve some interesting problems. Not the kind of problems they're
hyped for; but problems, still.

Citing Satoshi Nakamoto, "conventional currencies rely on the trust that central
banks won't debase them, but their history is full of breaches of that trust".
This is true: countries go through periods of crisis, money is printed by
humans. No form of human-controlled money has lasted more than a few centuries.
A global currency that isn't controlled or affected by human actions and
emotions is very stable on the long term. And there is some value in that
longevity factor. Bitcoin solves the problem of having a digital currency with
fixed rules that are forever independent of human politics and emotions, and,
even if you don't care about it, that is still a valid problem to solve.

As for applications, having a place where anyone can host forums, files, games,
in such a manner that can't be censored by anything is also a comfortable
safeguard in the case things go terribly wrong in the world; even if they
probably won't. Ethereum is such a place, but what is the point of an eternal
application if it only lasts until a bug is found? Formal verification is the
act of mathematically proving that given software has zero bugs, and that goes
really well with the idea of eternal apps. That is why Kindelia is valuable. It
solves the problem of deploying general-purpose software in a computer that will
run these programs with the same well-defined rules and a never-ending, 100%
uptime, independent of any human action. Perhaps that problem isn't interesting
to most people, but it is still valid problem to solve.

For example, the notion of virtual worlds that can't ever be turned off is
pretty attractive to a gaming nerd like me. I can't leave a legendary sword on
World of Warcraft to my grandchildren. After all, who knows if Blizzard will
even exist by then? But I surely can do so on a virtual world hosted on
Kindelia.  The fact an item will never cease to exist makes it more valuable
today, which results in markets where digital assets are valued just like
physical assets, which is really cool. Kindelia's cheap states lets these
virtual worlds exist without complicated layer 2 indirections.

In short, Kindelia is a functional, worldwide computer, capable of hosting
eternal applications that can't ever be altered or turned off; not by their
owners, not by governments. And, with formal verification, these applications
can remain bug-free during their endless lifespans. Kindelia and Ethereum are
distributed, but that is meant to maximize uptime and security, not performance.
They aren't meant to beat cloud servers in raw throughput, and anyone who thinks
that simply doesn't get the point. But only these can grant, to their hosted
applications, the power of living forever, and that's the whole point that makes
them valuable. Perhaps not as valuable as the hype and price tickers let us
believe. But who is to blame, the tech, or the greed of speculative markets?

## Why not include a currency?

Many criticize crypto projects that end up creating an associated token, even in
cases where such a token isn't necessary, because their network already provides
one that would serve the purpose. But why nobody raises the same criticisms for
crypto computers, like Ethereum? Do they actually need a token? We don't think
so, and, if that is the case, then including one would be as dishonest as most
utility tokens.

One may wonder how an Ethereum-like network could operate without block rewards
and miner fees, which are the core reasons it needs a native token, but both
problems have a simple solution. For block rewards, user-deployed currencies and
apps can implement a "Reward" functionality that grants tokens and assets to the
current block miner:


```c
// CatCoin kindly grants 100 cat tokens to each block miner!

!(CatCoin action) {
  ...

  // If this block > last_mined_block:
  //   grants the block miner #100 tokens
  (CatCoin {Reward miner}) =
    !gblk block_num
    !bind last_mint = (CatCoin.get_last_mint)
    if (> block_num last_mint) then
      !eval (CatCoin.send_tokens #100 miner)
      !eval (CatCoin.set_last_mint block_num)
      !done #0
    else
      !done #0
  
  ...
}

// To collect his reward, the block miner just adds an unsigned
// run{} block with all the rewards he wants to collect:

run {
  !call ~ (CatCoin {Reward 'Alice'})
  !call ~ (FurArena {Reward 'Alice'})
  !call ~ (FrogSwap {Reward 'Alice'})
  !done #0
}
```

That way, miners get not just ETH, or just BTC, or any single token, but rather
a constellation of gifts from built-in apps, as a kind incentive for them to
keep contributing to the network's security. Everyone wins on this exchange.

In a similar fashion, users can attach miner fees by including an extra payment
at the end of their transactions. For example:

```c
// A run statement signed by Bob, who can't mine a block
run {

  // Do whatever Bob wants to do
  !call ~ (WorldOfCat {Cast 'TailWhip'})
  !call ~ (AlpacaSwap {Buy 'AlpaCoin' #50 #666})
  !call ~ (LionChurch {Marry 'Alice'})

  // Pays 42 Kold coins to the miner
  !call miner (BlockMiner {Get})
  !call ~     (Kold {Send #42 miner}) // pls include me, thx bye

  !done #0

} sign {
  000123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
}
```

Bob wants to interact with 3 apps, but don't have the time or resources to mine
his own block. As such, he writes a `run{}` statement, performs the desired
transactions and, before ending, gives 42 Kold coins to the block miner. He then
signs the statement and broadcasts it to the network. Miners will be incentived
to include it in order to collect the Kold coins, in the exact same way they're
incentived by Ether fees, except replacing the native, hardcoded token, by any
user-submitted assets.

If Kindelia doesn't need a native token to exist, then it won't have a native
token, because that's how our ethics work - simple as that. But, then, how do we
raise funds? That is a separate question, which, we believe, must have a
separate answer. Ethereum, for example, minted and sold, out of thin air, more
than 50% of its total supply, before the network was released, and an additional
5% was granted to the Ethereum Foundation. We don't think that is wrong: whoever
designed the network has the right to dictate its rules, and there is nothing
wrong with that. Furthermore, from that presale came all the funding that
allowed Ethereum to exist, and that is a great thing. But, on the long term,
that agressively centralized distribution isn't aligned with the decentralized
vision it sells, and, as such, I intend to make it differently.

Kindelia Foundation, the non-profit entity, will launch a token on Kindelia,
which will be used to fund its future developments, including the maintenance of
the network, ecosystem apps, as well as next-gen HVM compilers and processors.
But that token will be launched as a normal app, under the same conditions as
every other, with no privileged position on the network's code. If the token
fails, then we'll not be able to keep working, but the network will still exist,
independent of its creators, as long, of course, as other developers, miners and
users decide to keep it alive. That said, we believe we're doing a superb job,
and hope Kindelia doesn't fire us! :)

## Why Nakamoto Consensus (Proof of Work)?

One of the main design goals of Kindelia is to have a simple, small reference
implementation, promoting client diversity and decreasing our roles as
figureheads. Blockchain projects spend too much effort trying to achieve
technical decentralization, but not enough of them care about political
decentralization. Proof of stake algorithms, while obviously superior in
theory - after all, having the option, why would anyone choose to waste energy? - are,
compared to proof of work, very hard to implement. Maintaining a PoS core would
greatly increase the network's complexity, decrease client diversity, increase
attack vectors, and require more core developers, increasing their power. All
these things go against the very philosophy of Kindelia, as it aims to be the
most stable, robust, decentralized computer conceivable.

Furthermore, despite the greatly increased complexity, every PoS algorithm
designed to date is strictly weaker than PoW, providing less guarantess. For
example, in [a
comment](https://ethresear.ch/t/explaining-the-liveness-guarantee/4228/3) about
50/50 netsplits, Vitalik Buterin notes that such events are extremelly unlikely,
and that protecting the network against them is overkill. I agree with his
sentiment, and I believe, and hope, that humanity will never find itself in a
situation where two sides of the world go completely out of communication for
weeks due to some catastrophic event. But if that does happen, I believe we
should have at least one ultra-resilient digital economy capable of recovering.
Kindelia fills that role, being the back-up plan, meteor-proof computer that,
along Bitcoin, will still be there in case everything goes wrong.

Finally, there is a common misconception that proof of stake increases a
network's layer 1 scalability, efficiency and throughput. That is simply not
true. These factors are capped on how efficiently the client uses the
computation, memory, disk and bandwidth that a node has available. Kindelia
addresses all these concerns directly, with a fast virtual machine, cheap memory
writes and reads, a limit on disk growth, and blocks that fit in a single UDP
packet. The main benefit of proof of stake is decreasing energy consumption
which, with sadness in our hearts, we do not know how to address right now. But
Kindelia should absolutely migrate to proof of stake one day, if an algorithm is
proven to be secure and simple enough not to impact the network's robustness.

## Why not become a layer 2 Ethereum rollup?

When it comes to scalability, everyone, their parents and parrots has an
opinion. Let's begin with a hard fact: no decentralized layer 1 computer will
ever achieve VISA-level scalability. That is not a pessimistic projection, but a
direct consequence of the overhead of consensus, the size of Earth and the speed
of light. Because of that, layer 2 solutions are proposed to improve throughput,
allowing computations to take place off-chain, in such way that that only the
final result, plus some proof, must be submitted to the principal network. This
increases scalability considerably.

With that in mind, one might ask: if layer 2 solutions increase scalability, and
if the HVM is also meant to increase scalability, then why not just make HVM an
Ethereum layer 2? That question makes as much sense as asking why don't running
shoes make pasta cook faster. These are categorically different things. Layer 2
solutions are algorithms that allow the network to find the result of certain
computations without having to run them to begin with. HVM is a way to actually
run these computations faster.

### Why even bother with a fast layer 1?

A question that does make sense, though, is: if layer 2 is the only long-term
solution for scalability anyway, then why even bother with a faster layer 1? The
thing is, even if that is the case, layer 2 solutions still rely on the layer 1
for disputes, so, all things equal, a faster, safer layer 1 will result in
better second layers. Moreover, while there are some great layer 2 solutions,
truth is, no matter how well designed some of them are, there is a common issue:
they all make some kind of compromise or impact the user experience in some way.
Moving apps or computations in and out is an unavoidable overhead. If an app
could operate efficiently on the layer 1, it would never resort to layer 2. A
faster layer 1 increases the set of apps that don't need to be moved out.
Dynamic MMORPGs and formally verified Idris2 apps that would require a layer 2
can have the option of living natively on Kindelia's layer 1, and having an
option is never a bad thing.

### On optimistic rollups

For example, optimistic rollups are just a fancy way of saying that someone will
compute the app's state outside and send it to the network, which will blindly
trust that it is correct. If the submitter lies and someone notices, a dispute
mechanism is triggered, halting the app until it is resolved, and punishing the
lier. The clever bit is that the fact the dispute mechanism exists means it is
almost never used, allowing most computations to be performed offchain.

This is a great idea that works very well in practice, but it brings some nasty
complications. The fact someone has complete write control over an app's state
is dangerous. If a fraud goes unnoticed, that person can to do anything with the
app's memory. Not even 51% attacks have that much destructive power. A network
dominated by optimistic rollups is inherently less robust than a traditional
layer 1, where computations are independently validated by every node. The
question is: is it worth? Let's see the numbers.

From [@corwintines's
article](https://ethereum.org/en/developers/docs/scaling/optimistic-rollups/#:~:text=Optimistic%20rollups%20sit%20in%20parallel,or%20%22notarise%22%20the%20transaction.)
on [ethereum.org](ethereum.org), optimistic rollups are claimed to provide
10-100x improvements in scalability. In our [comparison
table](#comparison-table), though, we claim HVM apps offer 10-434x improvements
over EVM apps, natively on layer 1, without any kind of rollups! In other words,
due to the sheer performance of the HVM, Kindelia apps are as scalable as layer
2 Ethereum apps in that case, without the dangerous compromise of someone having
full write access over app states. And a layer 2 on top of Kindelia would
possibly increase that 10-100x further. It seems like Ethereum spent an lot of
effort thinking in ways to avoid the EVM, because it is very slow. Kindelia cuts
all the trouble by just replacing it with something much faster to begin with.
It is depressing to think humanity could live forever with a terribly inefficient
virtual machine as the backbone of its worldwide economy, just because it is too
late to replace it.

### On zero knowledge proofs

A different, more cutting-edge solution, is that of zero-knowledge proofs, which
allow replacing the dispute mechanism by a cryptographic proof that the computed
result is correct. A layer 2 based on zk-starks would greatly increase the
network throughput. In fact, zk-snarks could even be attached to the layer 1, so
that a layer 2 wasn't even be needed. Users could submit the result of their
transactions directly, and nodes would just update the network state, without
even running the EVM. This idea is amazing and, if delivered properly, would,
honestly, be inherently superior to running apps in a virtual machine, like the
HVM. There is no free lunch, though. This idea has its own complications.

First, zero knowledge proofs are new, unstandardized, complex cryptographic
primitives. If any vulnerability is found in them, the network will be doomed.
Moreover, their sheer complexity make the protocol much harder to implement and
understand. Kindelia's reference node has less than 10k lines of code, and its
computation rules are all 5-20 lines long. This makes it easy to understand, and
promotes client diversity, allowing independent parties to implement their own
nodes, decreasing our roles as lead developers and figureheads. Decentralized
projects spent a lot of money trying to achieve technical decentralization, but
not enough of them account for political decentralization, resulting in core
developers being in a privileged position of power. In that sense, a project
based on zero knowledge proofs will never be as decentralized as a simpler
network which anyone can audit, implement and understand.

Second, one of the most important features of decentralized projects is the fact
users can manually audit all the computations that resulted in a specific
balance or state, and zk-proofs prevent that. Compare, for example, Bitcoin to
zCash. On Bitcoin, anyone can easily navigate the entire history of a bitcoin
all the way to the block where it was minted, and the circulating supply is
crystal clear. On zCash, this isn't possible at all. There could be billions of
ZEC coins generated from an exploit lying in someone's wallet, and nobody would
notice. In the case of decentralized computers, zero knowledge proofs would do
that, except to the entire network state.

Third, computation isn't the only factor impacting scalability. In fact, it
isn't even the most relevant. SSTORE is, by far, the most problematic operation,
and it is expensive because it increases the state size. Nor zero knowledge
proofs, nor HVM, can do anything about that. Now, reused SSTOREs, i.e.,
rewriting state without growing the total size, can be massively improved. HVM
does so it by replacing merkle trees by reversible heaps, which are just cheap
layered buffers. An hypothetical zero knowledge network could do the same. As
for Ethereum, its layer 1 already relies on Merkle trees, so, unless it replaces
the entire store machinery, dynamic apps like virual game worlds will never be
viable on layer 1.

### Conclusion

In short, while layer 2 solutions are the only way to achieve VISA-level
scalability, they come with unavoidable complications and compromises.
Optimistic rollups require complex dispute mechanisms with terrible worst-case
scenarios, zero knowledge proofs rely on unproven cryptography, decrease
transparency and client diversity, and don't address the other half of the
scalability bottleneck, which is not computation, but state growth. There is
value in an efficient, secure, simple layer 1 computation network. Kindelia aims
to maximize all these aspects, as far as theory allows.
