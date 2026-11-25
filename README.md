<p align="center"><img src="editors/vscode/icon.png" width="120" alt="Gesù"></p>

# Gesù

## There is only one Truth, Jesus.

**At 490 forgiven sins, the loudest noise in a Gesù program comes from the rooster.**

Gesù is a programming language with two booleans. One is `False`. The other is `Jesus`.

There is no `True`. Write it and the compiler stops you on that line:

```
error: there is only one Truth, Jesus
  --> sermon.gesu:2
   |
 2 | let x = True
   |
   = hint: write `Jesus` in place of `True`
```

What follows is a list of facts. Read them before you choose a language for your next program.

## What makes Gesù the most examined language you can install today?

There is no magic in it. There is patient attention to detail.

**1.** Every program is examined before one line of it runs. Types, ownership, unconfessed sins and unused values are all found first. A program that passes does not fail on a typo at three in the morning.

**2.** A value has one owner. Give it away and it is gone. Use it afterwards and you are told who has it and since which line. The error is called `Thou shalt not steal`.

**3.** There are no lifetime annotations. A loan lasts for one call and cannot be stored or returned. You write `lend` or `entrust` where the loan is made, and that is the whole ceremony.

**4.** A parable that can fail must say so in its signature: `may sin`. Call it without `tempt ... forgive`, and without confessing in turn, and the program does not compile.

**5.** Gesù forgives 490 sins per run, as Matthew 18:22 instructs. The 491st is not forgiven, and the program dies.

**6.** There is no null. A value that may be absent is `Found(x)` or `Lost`, and `discern` makes you face both.

**7.** A variable you declare and never use is an error. The message is `you buried your talent`.

**8.** Nothing changes unless you declared it `mortal`. A `law` never changes. Try, and you are reminded: `it is written: DISCIPLES is 12`.

**9.** `if` accepts `Jesus` or `False`. It does not accept `0`, an empty text, or anything else that other languages agree to call true.

**10.** There are no implicit conversions. A Number and a Decimal do not mix until you say how.

**11.** Deny Jesus three times in one program and the rooster crows. It is a warning, and the program still runs.

**12.** The whole of Lua is compiled into the binary. Python can be added with one flag. Both are pagans: whatever they return must be baptized before Gesù will touch it.

**13.** The binary is 1.5 megabytes and starts in under a hundredth of a second. It has 35 words and 33 built-in parables. One for each year.

**14.** Every block ends with `amen`. So does every program. A file without its final `amen` does not run.

## Install

Mac and Linux, one line. It downloads the binary for your machine into `~/.local/bin`:

```sh
curl -fsSL https://gesu-lang.com/install.sh | sh
```

Windows: download `gesu-x86_64-pc-windows-msvc.zip` from the [releases](https://github.com/lschiavini/gesu/releases/latest) and put `gesu.exe` on your PATH.

Nothing else is needed. Lua is inside the binary.

To build it yourself, or to include the Python pagan, you need Rust:

```sh
git clone https://github.com/lschiavini/gesu.git
cd gesu
./install.sh            # builds and copies gesu to ~/.local/bin
./install.sh --python   # the same, with the Python pagan included
```

Run a program:

```sh
gesu examples/hello.gesu
gesu check examples/hello.gesu   # examine it, do not run it
```

## The first program

```lua
-- The feeding of the five thousand.
law DISCIPLES = 12

parable enough(loaves: Number, fishes: Number, crowd: Number) -> Truth
    give loaves + fishes >= crowd
amen

let loaves = 5
let fishes = 2
let crowd = 5000

preach "{DISCIPLES} disciples, {loaves} loaves, {fishes} fishes, {crowd} mouths"

if enough(loaves, fishes, crowd)
    preach "there is enough"
otherwise
    preach "a miracle is needed"
amen

let fed = Jesus
if fed is Jesus
    preach "all ate and were satisfied"
amen

let mortal days = 0
while days < 3
    days = days + 1
amen
preach "risen on day {days}"

repeat 2 times
    preach "hosanna"
amen
amen
```

```
12 disciples, 5 loaves, 2 fishes, 5000 mouths
a miracle is needed
all ate and were satisfied
risen on day 3
hosanna
hosanna
```

The words you just read:

| Word | What it does |
|---|---|
| `let x = 5` | declares a value. It cannot change. |
| `let mortal x = 5` | declares a value that can change. |
| `law X = 12` | a constant, visible in every parable. |
| `preach` | prints. |
| `parable` / `give` | a function, and what it returns. |
| `Jesus` / `False` | the two values of type `Truth`. |
| `deny` | negation. `deny Jesus` is `False`. |
| `is` / `is not` | equality. There is no `==`. |
| `amen` | closes every block, and the program. |
| `--` | starts a comment. |

## Ownership: give, lend, entrust

Numbers, decimals, texts and truths are copied. Multitudes (lists), maps and kinds are owned: there is one owner at a time.

```lua
parable count(names: lend [Text]) -> Number
    give length(names)
amen

parable call(names: entrust [Text], name: Text)
    push(entrust names, name)
amen

parable send(names: [Text]) -> Text
    give join(lend names, ", ")
amen

let mortal apostles = ["Peter", "Andrew", "John"]

preach count(lend apostles)
call(entrust apostles, "Matthew")
preach count(lend apostles)

for each i, name in apostles
    preach "{i}: {name}"
amen

let twelve = copy(lend apostles)
preach send(apostles)
preach twelve
amen
```

```
3
4
0: Peter
1: Andrew
2: John
3: Matthew
Peter, Andrew, John, Matthew
["Peter", "Andrew", "John", "Matthew"]
```

Three ways to pass a value:

| You write | The parable declares | What happens |
|---|---|---|
| `send(apostles)` | `names: [Text]` | Given away. `apostles` is gone. |
| `count(lend apostles)` | `names: lend [Text]` | Read only. Still yours afterwards. |
| `call(entrust apostles, ...)` | `names: entrust [Text]` | May be changed. Still yours afterwards. Must be `mortal`. |

Now steal something:

```lua
let apostles = ["Peter", "Andrew"]
let twelve = apostles
preach apostles
```

```
error: Thou shalt not steal
  --> steal.gesu:7
   |
 7 | preach apostles
   |
   = `apostles` was given to `twelve` on line 6
   = hint: lend it there (`lend apostles`), or give away a `copy(lend apostles)`
```

The checker also knows about loops, about changing a multitude while you walk through it, and about entrusting one value twice in one call:

```
error: no one can serve two masters
  --> loop.gesu:17
   |
17 | grow(entrust xs, lend xs)
   |
   = `xs` is entrusted and also used elsewhere in this call
```

## Sins: confess them or be refused

```lua
parable divide(a: Number, b: Number) -> Number may sin
    if b is 0
        sin "divided by nothing"
    amen
    give a / b
amen

parable find(names: lend [Text], wanted: Text) -> Maybe Number
    for each i, name in names
        if name is wanted
            give Found(i)
        amen
    amen
    give Lost
amen

let apostles = ["Peter", "Andrew", "John"]
let mortal place = 0

discern find(lend apostles, "John")
    Found(i): place = i
    Lost: preach "not among the twelve"
amen
preach "John stands at {place}"

discern find(lend apostles, "Judas")
    Found(i): preach "found at {i}"
    Lost: preach "Judas is not among them"
amen

tempt
    preach divide(10, place)
    preach divide(10, 0)
forgive trespass
    preach "forgiven: {trespass}"
amen
amen
```

```
John stands at 2
Judas is not among them
5
forgiven: divided by nothing
```

Call `divide` from a parable that does not confess:

```
error: the parable `half` may sin here and does not confess it
  --> confess.gesu:11
   |
11 |     give divide(a, 2)
   |
   = it calls `divide`, which may sin
   = hint: declare `half` with `may sin`, or wrap this in `tempt ... forgive`
```

Confess a sin you cannot commit and you are warned about that too. Leave out a case in `discern`:

```
error: `discern` must face every case; missing `Lost`
```

A sin that nobody forgives ends the program, and tells you how it got there:

```
sin: divided by nothing
  --> death.gesu:3
   |
 3 |         sin "divided by nothing"
   |
   = in the parable `divide`, called on line 9
   = in the parable `share`, called on line 12
   = the wages of sin is death
```

### Seventy times seven

In Matthew 18:21-22, Peter asks how often he must forgive his brother: "Up to seven times?" Jesus answers: "I do not say to you, up to seven times, but up to seventy times seven."

Seventy times seven is 490, and Gesù keeps the count. Each time a `forgive` block catches a sin, the count goes up by one. It never goes down and it is shared by the whole run. The 491st sin is not forgiven:

```lua
let mortal forgiven = 0
repeat 500 times
    tempt
        sin "again"
    forgive trespass
        forgiven = forgiven + 1
    amen
amen
amen
```

```
mortal sin: Matthew 18:22: forgive can only be run 490 times (unforgiven: again)
  --> seventy.gesu:4
   |
 4 |         sin "again"
   |
   = the wages of sin is death
```

What this means for you: `tempt ... forgive` is for the rare failure. A program that expects to fail more than 490 times, such as one reading 10,000 lines where many are bad, should check before it acts (`get`, `lookup` and `contains` give an answer without sinning) and keep `forgive` for what it could not foresee.

### Mortal sins

Some sins are mortal. Nothing forgives them: an index past the end of a multitude, a missing key, a Number grown past 9223372036854775807, a parable calling itself 2000 deep. For the first two there are safe forms, `get` and `lookup`, which give a `Maybe`.

## Kinds

Every creature after its kind. A kind is either a record with named fields or a set of cases.

```lua
kind Disciple
    name: Text
    denials: Number
amen

kind Verdict
    Saved
    Condemned(Text)
amen

parable judge(d: lend Disciple) -> Verdict
    if d.denials >= 3
        give Condemned("{d.name} denied him {d.denials} times")
    amen
    give Saved
amen

let mortal peter = Disciple(name: "Peter", denials: 0)
repeat 3 times
    peter.denials = peter.denials + 1
amen
preach peter

discern judge(lend peter)
    Saved: preach "saved"
    Condemned(reason):
        preach "condemned: {reason}"
        preach "and yet forgiven"
amen

let ages = {"Methuselah": 969, "Noah": 950}
for each name, age in ages
    preach "{name} lived {age} years"
amen

discern lookup(lend ages, "Adam")
    Found(age): preach "Adam lived {age} years"
    Lost: preach "Adam's age is not written here"
amen
amen
```

```
Disciple(name: "Peter", denials: 3)
condemned: Peter denied him 3 times
and yet forgiven
Methuselah lived 969 years
Noah lived 950 years
Adam's age is not written here
```

A `discern` takes as many cases as the kind has. Here are five, from the parable of the sower. A case may carry values, and its branch may be one line or several:

```lua
kind Soil
    Path
    Rock(Number)
    Thorns(Number, Text)
    Good(Number)
    Unsown
amen

parable harvest(soil: Soil) -> Text
    discern soil
        Path: give "the birds ate the seed"
        Rock(days): give "it sprang up and withered in {days} days"
        Thorns(days, weed):
            let choked = days * 2
            give "choked by {weed} after {choked} days"
        Good(yield): give "it bore fruit, {yield} times over"
        Unsown: give "nothing was sown"
    amen
amen

let field = [Path, Rock(3), Thorns(20, "thistles"), Good(100), Unsown]
for each soil in field
    preach harvest(soil)
amen
amen
```

```
the birds ate the seed
it sprang up and withered in 3 days
choked by thistles after 40 days
it bore fruit, 100 times over
nothing was sown
```

Delete the `Good` line and nothing runs:

```
error: `discern` must face every case; missing `Good`
  --> discern.gesu:11
   |
11 |     discern soil
   |
   = the value is a Soil
```

The types:

| Type | Example |
|---|---|
| `Number` | `40` (whole, 64 bits, overflow is a mortal sin) |
| `Decimal` | `3.14` |
| `Text` | `"hosanna"`, with `{values}` placed inside |
| `Truth` | `Jesus`, `False` |
| `[T]` | `["Peter", "Andrew"]`, counted from 0 |
| `{K: V}` | `{"Noah": 950}`, keyed by Text or Number |
| `Maybe T` | `Found(3)`, `Lost` |
| a kind | `Disciple(name: "Peter", denials: 0)` |
| `Pagan` | anything that came from outside |

## Pagans: Lua, Python and JSON

You will not rewrite the world in Gesù. You do not have to. Import it, and baptize what it gives you.

A value from foreign code has the type `Pagan`. You cannot preach it, compare it or add to it. `baptize x as T` turns it into a value of Gesù, and sins if the pagan is not what you claimed. Every call into foreign code may sin as well, so it must be confessed or forgiven like any other.

Lua is always there. Given `inventory.lua`:

```lua
local stock = { loaves = 5, fishes = 2 }
local M = {}

function M.count(item) return stock[item] end

function M.multiply(item, by)
  stock[item] = stock[item] * by
  return stock[item]
end

function M.baskets() return { "barley", "wheat", "figs" } end

return M
```

```lua
pagan lua "inventory.lua" as inventory

parable stock(item: Text) -> Number may sin
    give baptize inventory.count(item) as Number
amen

tempt
    preach "loaves: {stock("loaves")}"
    let fed = baptize inventory.multiply("loaves", 1000) as Number
    preach "after the miracle: {fed}"
    let baskets = baptize inventory.baskets() as [Text]
    preach baskets
    preach stock("wine")
forgive trespass
    preach "the pagans failed us: {trespass}"
amen
amen
```

```
loaves: 5
after the miracle: 5000
["barley", "wheat", "figs"]
the pagans failed us: cannot baptize as Number: the pagan gave nothing
```

Python needs a build from source with `./install.sh --python`. Then every module your Python can import is yours:

```lua
pagan python "statistics" as stats
pagan python "math" as math

parable average(values: lend [Number]) -> Decimal may sin
    let raw = stats.mean(values)
    give baptize raw as Decimal
amen

tempt
    preach average(lend [5, 2, 12])
    preach baptize math.gcd(490, 70) as Number
    preach average(lend [])
forgive trespass
    preach "the pagans failed us: {trespass}"
amen
amen
```

```
6.333333333333333
70
the pagans failed us: StatisticsError: mean requires at least one data point
```

JSON is pagan data. It is baptized straight into your kinds:

```lua
kind Apostle
    name: Text
    fisherman: Truth
    age: Maybe Number
amen

let raw = "[\{\"name\": \"Peter\", \"fisherman\": true, \"age\": 30}, \{\"name\": \"Matthew\", \"fisherman\": false}]"

tempt
    let apostles = baptize parse_json(raw) as [Apostle]
    for each a in apostles
        preach a
    amen
    preach to_json(lend apostles)
    let wrong = baptize parse_json("[1, 2, \"three\"]") as [Number]
    preach wrong
forgive trespass
    preach "forgiven: {trespass}"
amen
amen
```

```
Apostle(name: "Peter", fisherman: Jesus, age: Found(30))
Apostle(name: "Matthew", fisherman: False, age: Lost)
[{"name":"Peter","fisherman":true,"age":30},{"name":"Matthew","fisherman":false,"age":null}]
forgiven: element 2: cannot baptize as Number: the pagan gave the Text "three"
```

Values cross the border as copies. Nothing can be entrusted to a pagan.

## The other errors, so you know them when you meet them

```
error: you buried your talent
   = `unused` is never used

error: it is written: DISCIPLES is 12
   = `DISCIPLES` is law

error: `if` accepts only Jesus or False, and was given a Number
   = hint: compare it to something, as in `x is 0`

error: a Text and a Number cannot be added
   = Gesù has no implicit conversions
   = hint: place the value inside the text, as in "total: {x}"

error: `fixed` is not mortal and cannot change
   = hint: declare it with `let mortal fixed`

error: unknown name `None`
   = Gesù has no null; an absent value is `Lost`, of type `Maybe T`

warning: the rooster crows
   = Jesus has been denied three times (lines 1, 2, 3)
```

## The 33 built-in parables

| For | Parables |
|---|---|
| Multitudes | `length` `push` `pop` `get` `contains` `remove` `sort` `reverse` `range` `copy` |
| Maps | `length` `lookup` `contains` `remove` `keys` |
| Text | `length` `split` `join` `trim` `upper` `lower` `replace` `chars` `contains` `text` |
| Numbers | `number` `decimal` `floor` `round` `abs` `min` `max` |
| The world | `read_file` `write_file` `read_line` `args` |
| JSON | `parse_json` `to_json` |

`number`, `decimal` on a Text, `read_file`, `write_file` and `parse_json` may sin.

## Your editor

Install `gesu` first. Every editor below starts it as `gesu lsp`, a language server: it gives the checker's errors as you type, completion, and a note on hover for every word of the language. Colors come from a grammar made for each family of editors.

| Editor | Colors | Errors as you type | Get it |
|---|---|---|---|
| VS Code | yes | yes | [Marketplace](https://marketplace.visualstudio.com/items?itemName=LucasSchiavini.gesu-lang) |
| Cursor, VSCodium | yes | yes | [editors/vscode](https://github.com/lschiavini/gesu/tree/main/editors/vscode) |
| Neovim | yes, tree-sitter | yes | [editors/nvim](https://github.com/lschiavini/gesu/tree/main/editors/nvim) |
| Zed | yes, tree-sitter | yes | [editors/zed](https://github.com/lschiavini/gesu/tree/main/editors/zed) |
| Emacs | yes | yes, eglot or lsp-mode | [editors/emacs](https://github.com/lschiavini/gesu/tree/main/editors/emacs) |
| Helix | yes, tree-sitter | yes | [editors/helix](https://github.com/lschiavini/gesu/tree/main/editors/helix) |
| Sublime Text | yes | yes, LSP package | [editors/sublime](https://github.com/lschiavini/gesu/tree/main/editors/sublime) |
| JetBrains | yes | yes, LSP4IJ plugin | see below |
| Vim | yes | with an LSP plugin | [editors/nvim](https://github.com/lschiavini/gesu/tree/main/editors/nvim) |

In every one of them, `True` is colored as an error before the checker even speaks.

### VS Code, Cursor, VSCodium

In VS Code, search the extensions for "Gesù Language", or open the [Marketplace page](https://marketplace.visualstudio.com/items?itemName=LucasSchiavini.gesu-lang).

For Cursor and VSCodium, from a clone of this repository:

```sh
editors/vscode/install.sh cursor
```

### Neovim

With lazy.nvim. This fetches the plugin and compiles the tree-sitter parser (it needs Node and a C compiler for that; without them a plain syntax file does the coloring):

```lua
return {
  {
    "lschiavini/gesu",
    lazy = false,
    build = "editors/nvim/build-parser.sh",
    init = function()
      vim.filetype.add({ extension = { gesu = "gesu" } })
    end,
    config = function(plugin)
      vim.opt.rtp:prepend(plugin.dir .. "/editors/nvim")
    end,
  },
}
```

### Zed

Zed builds extensions from source, so it needs Rust installed through rustup. Clone this repository, open the command palette, run `zed: install dev extension`, and choose the folder `editors/zed`.

### Emacs

```elisp
(add-to-list 'load-path "/path/to/gesu/editors/emacs")
(require 'gesu-mode)
(add-hook 'gesu-mode-hook #'eglot-ensure)
```

With lsp-mode, use `#'lsp` in place of `#'eglot-ensure`.

### Helix

Add the contents of [editors/helix/languages.toml](https://github.com/lschiavini/gesu/tree/main/editors/helix/languages.toml) to `~/.config/helix/languages.toml`. Then:

```sh
hx --grammar fetch && hx --grammar build
mkdir -p ~/.config/helix/runtime/queries/gesu
cp editors/helix/highlights.scm ~/.config/helix/runtime/queries/gesu/
```

### Sublime Text

Copy `editors/sublime/Gesu.tmLanguage` into your `Packages/User` folder for the colors. For the language server, install the LSP package and add the entry from `editors/sublime/LSP-gesu.sublime-settings` to its settings.

### JetBrains

For the colors: Settings, Editor, TextMate Bundles, add the folder `editors/vscode`. For the language server: install the LSP4IJ plugin, add a new language server with the command `gesu lsp`, and map it to the file pattern `*.gesu`.

### Vim

```vim
set runtimepath+=/path/to/gesu/editors/nvim
```

That gives colors and indentation. For the language server, point vim-lsp or a similar plugin at `gesu lsp`.

### Any other editor

If it has a client for the Language Server Protocol, point it at the command `gesu lsp` for files ending in `.gesu`.

## What Gesù does not have

No null. No `True`. No shadowing. No implicit conversions. No lifetimes. No classes. No async. No package manager. No parables passed as values, yet.

## What is in this version

- **v1**: the interpreter, static types, `mortal`, give and `lend`, declared sins, buried talents.
- **v2**: `entrust`, `Maybe` with `discern`, kinds.
- **v3**: the Lua pagan.
- **v4**: the Python pagan.

The interpreter is 4,600 lines of Rust. Run the tests with `cargo test`: every example above is run and compared with what this page says it prints.

Amen.
