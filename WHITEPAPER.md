---
# mainfont: DejaVuSerif.ttf
# sansfont: DejaVuSans.ttf
# monofont: DejaVuSansMono.ttf
monofont: /Users/kelvin/Library/Fonts/Fira Code Regular Nerd Font Complete Mono.otf
# mathfont: texgyredejavu-math.otf
---

Kindelia: a peer-to-peer functional computer
============================================

Kindelia is a peer-to-peer functional computer capable of hosting applications
that stay up forever. It uses the [HVM](https://github.com/kindelia/hvm), a blazingly fast functional virtual
machine, to run formally verified apps natively, making it as secure as
mathematically possible. It also replaces expensive Merkle tree insertions by
reversible heaps snapshots, greatly reducing the cost of persistent state,
making highly dynamic apps economically viable. In short, Kindelia can be seen
as a massive simplification of Ethereum, with no native currency, greatly
increased security, and greatly reduced layer 1 transaction costs.

Introduction
------------

Kindelia is a pure decentralized computer. It aggressively purges redundant
features, resulting in a minimal 10K-LOC implementation, promoting client
diversity and addressing political centralization. It has no built-in token,
making it a non-cryptocurrency and addressing economical centralization. It
replaces the EVM, a slow stack machine, by the HVM, a fast graph reduction
machine, and slow Merkle trees by fast heap snapshots. These allow it to
host 2 kinds of apps that, previously, were economically unviable:

### 1. Formally Verified apps

These apps are developed in functional languages like Idris, Agda and Kind, and
accompanied by a "formal verification" of their correctness, which is like a
mathematical guarantee that they can not have any bug or exploit, at all.  This
is extremely valuable for DApps, because they hold money, yet can't be patched
or reversed, which is a recipe for disaster. Deploying formally verified apps
written on these languages to Ethereum or Cardano is not economically viable,
but Kindelia can handle these natively and cheaply.

### 2. Dynamic Stateful apps

Apps that involve huge amouts of state changes, such as games and exchanges, are
too expensive on Ethereum, due to the high cost of the SSTORE opcode. Moreover,
persisting state on Ethereum is complex and error-prone, due to the need of
serializing structures into a map of 256-bit integers. Kindelia replaces the
Merkle stores by reversible snapshots of HVM's heap, which makes the SSTORE
opcode essentially free. This not only makes statefulness much more convenient
and less error-prone (since apps can persist high-level structures), but also
allows Kindelia to host highly dynamic layer 1 apps, such as games, which are
simply not viable on Ethereum.

In short, for these familiar with functional languages, Kindelia can be seen as
a worldwide functional REPL (like Haskell's GHCI) where anyone can input
commands that deploy functions and run monadic actions forever. And for these
familiar with decentralized computers, Kindelia can be seen as a polishment of
Ethereum, under the following recipe:

```
1. Start from Ethereum
2. Remove Ether (and the associated pre-mine)
3. Replace the slow EVM by the fast HVM
4. Replace slow Merkle trees by fast reversible heaps
5. Aggressively optimize and simplify everything you find
6. Cook without hype
7. Serve with a fair release
```

How it works?
-------------

On conventional cryptocurrencies such as Bitcoin, network peers use a consensus
algorithm to agree on a canonical ordering of blocks, which group user-generated
transactions, which have the effect of sending money from an address to another.
On Ethereum, these transactions can also call execute computations that change a
contract's state. On Bitcoin, the network state is just a global map of
balances. On Ethereum, each contract has its own state, which is a map of uints.
On Kindelia, blocks don't group *transactions*, but *statements* with no
monetary information attached; that is, statements don't have "to" or "amount"
fields. Instead, they are just code blocks that affect the network state. Since
there is no native currency, nodes use Proof-of-Work consensus to order blocks.

     Block #0                  Block #1                  Block #2
    .---------------.         .---------------.         .---------------.
    | - Prev_Hash   | <------ | - Prev_Hash   | <------ | - Prev_Hash   |
    | - Time        |         | - Time        |         | - Time        |
    | - Nonce       |         | - Nonce       |         | - Nonce       |
    | - Statement_0 |         | - Statement_0 |         | - Statement_0 |
    | - Statement_1 |         | - Statement_1 |         | - Statement_1 |
    | - ...         |         | - ...         |         | - ...         |
    | - Statement_N |         | - Statement_N |         | - Statement_N |
    '---------------'         '---------------'         '---------------'

There are 4 kinds of statements:

- **CTR**: declares a new type (constructor)

- **FUN**: install a new global function

- **RUN**: runs a side-effective script

- **REG**: registers a namespace

Kindelia functions are defined by equations, as in Haskell. For example, the
function below sums an immutable tree:

```c
fun (Sum tree) {
  (Sum {Leaf x})     = x
  (Sum {Branch a b}) = (+ (Sum a) (Sum b))
}
```

Once deployed and mined, this statement will cause `Sum` to be defined globally.
Kindelia functions can call each-other, and are pure, thus, side-effect free.
The `RUN` statement offers a scape hatch where side-effects can occur and alter
the network's state, just like Haskell's IO. For example, the statement below
adds `3` to the state stored by the global 'Calculator' contract:

```c
run {
  !call ~ 'Calculator' [{Add 3}]
  !done #0
}
```

When that statement is included in a block, its actions are evaluated inside the
[HVM](https://github.com/kindelia/hvm), which offers all the features required
for a great decentralized runtime: instant compilation, no garbage accumulation,
great performance, and a well-defined cost table. Among alternatives, only
Kindelia is capable of running functional apps with an efficiency on par with
Haskell's GHC.

```
.-----------------------------------------------------.
| Opcode    | Effect                          | Mana  |
|-----------|---------------------------------|-------|
| APP-LAM   | applies a lambda                | 2     |
| APP-SUP   | applies a superposition         | 4     |
| OP2-NUM   | operates on a number            | 2     |
| OP2-SUP   | operates on a superposition     | 4     |
| FUN-CTR   | pattern-matches a constructor   | 2 + M |
| FUN-SUP   | pattern-matches a superposition | 2 + A |
| DUP-LAM   | clones a lambda                 | 4     |
| DUP-NUM   | clones a number                 | 2     |
| DUP-CTR   | clones a constructor            | 2 + A |
| DUP-SUP-0 | clones a superposition          | 4     |
| DUP-SUP-1 | undoes a superposition          | 2     |
| DUP-ERA   | clones an erasure               | 2     |
|-----------------------------------------------------|
| * A is the constructor or function arity            |
| * M is the alloc count of the right-hand side       |
'-----------------------------------------------------'
```

The elegant cost table above exhibit the 12 opcodes of HVM, including lambda
application, pattern-matching, integer operations and primitives for erasing and
duplicating data. These opcodes are expression-based, rather than stack-based.
For example, an `OP2-NUM` is counted on the `(+ 2 3) ~> 5` computation. Kindelia
nodes impose a maximum "mana" limit per block, which combats spam, fulfilling a
role similar to Ethereum's gas.

Note that the table above has only computational opcodes; there is no equivalent
to "SSTORE" or "SLOAD". That's because space is treated separately from
computation. In order to persist states, functions can invoke the `save` and
`load` actions. For example, the function below stores a state that can only be
incremented:

```c
fun (Increment) {
  (Increment) =
    !load num       // loads the stored number
    !save (+ num 1) // stores the number + 1
    !done #0        // returns 0
} with {
  #0 // initial state  
}
```

As usual, it can be called inside `run` statement. For example, the statement
below increments the state of the `'Increment'` function `3` times:

```
run {
  !call ~ 'Increment' []
  !call ~ 'Increment' []
  !call ~ 'Increment' []
  !done #0
}
```

Internally, all that `!save` does is store a pointer to the saved expression,
preventing it from being garbage-collected on HVM's runtime heap, which causes
it to persist across blocks. In other words, Kindelia's `!save` is extremely
cheaper than Ethereum's `SSTORE`, which involves an expensive Merkle tree
insertion. Not only that, apps can save and load entire structures such as
lists, trees, JSONs in a single call, instead of having to serialize and
deserialize them into 256-bit integers.

Of course, since `!save` has no cost, this would expose the network to a trivial
spam attack, where a function persists an obscene amount of data. To prevent
that, nodes also impose a limit on how much the HVM's heap can grow on each
block. That limit is called `bits`, and is separated from the `mana` limit. 

Statements can also be signed:

```c
run {
  !call ~ 'CatCoin' [{Send 'Alice' 100}]
  !done #0
} sign {
  00c0777281fe0a814d0f1826ad
  7f4228f7308df5c4365f8dc577
  ed64b3e32505a143d5566b8d38
  1f5b93988d19a82924fcef232e
  6ccc5a0e006e5b6f946cd15372
}
```

A signed statement works as usual, except that called functions can track the
caller's address, allowing it to operate like a contract. For example, a
`CatCoin` function can store an `Address -> Balance` to operate like a token.
Kindelia addresses consist of the first 15 bytes of the respective Ethereum
address. As such, both account systems are compatible, and Ethereum users can
use their existing accounts to sign Kindelia statements.

Finally, Kindelia has a built-in namespace system based on a hierarchy of names.
Kindelia functions are addressed by 72-bit names, a limit imposed by the size of
HVM's pointers. These names contain up to 12 6-bit chars, including letters,
numbers, underscore and periods. Names without periods can be deployed by
anyone. Names with periods are namespaced, and can only be deployed by the
namespace owner. So, for example, `Foo.Bar.app` can only be deployed by the
owner of the `Foo.Bar` namespace. The owner of a namespace can grant
sub-namespaces to other users.

Benchmarks
----------

Ethereum and Kindelia are have some architecturally different in some aspects,
so it is not always straightforward to draw direct comparisons between them,
but we can make approximations. The table below compares both networks
computationally:

```
Operation              |      Ethereum |       Kindelia |       ratio
---------------------- | ------------- | -------------- | -----------
Numeric addition       |  769,230 op/s | 5,000,000 op/s |       650 %
Numeric multiplication |  461,538 op/s | 5,000,000 op/s |     1,080 %
Lambda application     |   11,538 op/s | 5,000,000 op/s |    43,330 %
Pattern match          |   11,538 op/s | 5,000,000 op/s |    43,330 %
Uint load              |   23,076 op/s | 5,000,000 op/s |     4,333 %
Uint store             |      461 op/s | 5,000,000 op/s |   216,900 %
```

All the HVM opcodes have about the same performance (5 million operations per
second) because they require exactly one 2-mana HVM graph rewrite, with no other
significant costs. On Ethereum, additions and multiplications are cheap, because
they require cheap 3 to 5 gas instructions; applications and pattern matches are
expensive, because they require a several instructions, which amount to about
200 gas; loads and stores are expensive, because the SSTORE/SLOAD instructions
demand costly Merkle tree manipulations, and range from 100 to 20000 gas.
Kindelia also has considerably shorter transaction sizes. The table below
compares the size of deploying, and calling, a simple incrementer contract:

```
Operation           | Ethereum  | Kindelia | ratio
------------------- | --------- | -------- | -----
Deploy Foo contract | 550 bytes | 66 bytes |  12 %
Call inc() method   | 113 bytes | 32 bytes |  28 %
```

The difference is expressive, specially because these transactions don't require
signatures. Even if it did, Kindelia would still come shorter. This allows
Kindelia to have much smaller blocks without affecting its throughput, which, in
turn, allows it to fit a full block in a single 1500-byte UDP packet, which
makes block propagation as fast as the internet allows, decreasing the block
time to just 1 second, without impacting uncle rates significantly. The source
for these numbers is availble on [this Gist](https://gist.github.com/VictorTaelin/bb0f8fb30b61bf0c216675791b72500c).

Conclusion
----------

With pure functions, stateful actions and signed transactions, Kindelia can host
any conceivable application, from tokens, to exchanges, to complete virtual
worlds. Thanks to HVM's native functional opcodes, applications hosted on it can
be made as secure as mathematically possible, by programming them on well-typed
languages, or even proof assistants. Moreover, thanks to reversible heap
snapshots, Kindelia is able to host dynamic applications such as games and
exchanges natively, which wasn't economically possible without layer 2
solutions. 
