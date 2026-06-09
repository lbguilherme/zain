# CDP Domain Implementation Guide

Reference implementation: `browser.rs`

> **Codegen.** Domains listed in `build.rs`'s `MANIFEST` (currently `io`, `input`)
> are **generated** from their `{domain}.json` by `build.rs` following the rules
> below — do not hand-edit those `.rs` files; edit the JSON (or `build.rs`) and
> rebuild. The generator covers newtypes, object structs, string enums (incl.
> inline enums hoisted from params), events, optional fields, `Default`
> derivation, the direct-args shortcut, deprecated-command skipping and Rust
> keyword escaping. Not yet handled (so still hand-written): recursive types
> (`Box`, e.g. `DOM.Node`) and the cross-domain cluster that homes shared IDs in
> `crate::types` (`target`/`browser`/`page`/`dom`). The rest of this guide is
> both the hand-writing spec and the generator's spec.

## File structure

Each domain lives in its own `{domain}.rs` file (lowercase) and is re-exported in `mod.rs`.

Sections within the file, in order:

```
use ...

// ── Types ──────────────────────────────
// ── Param types ────────────────────────
// ── Return types ───────────────────────
// ── Events ─────────────────────────────
// ── Domain trait ───────────────────────
// ── Impl ───────────────────────────────
```

## Naming

| JSON | Rust |
|---|---|
| Domain `Foo` | File `foo.rs`, trait `FooCommands` |
| Command `doThing` | Method `foo_do_thing` |
| Command params | `DoThingParams` struct |
| Command return | `DoThingReturn` struct |
| Event `thingHappened` | `ThingHappenedEvent` struct |
| Type `SomeType` | `SomeType` struct/enum |
| Inline enum (inside a command param) | Dedicated enum ao lado dos types |

## Types

- Newtypes para IDs: `pub struct FooId(pub String)` com `Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash`.
- Integer newtypes (ex: `WindowId`): `pub struct WindowId(pub i64)` com `Copy` adicional.
- String enums: `#[serde(rename_all = "camelCase")]` enum com variantes PascalCase.
- Object types: struct com `#[serde(rename_all = "camelCase")]`.
- Newtypes compartilhados entre domínios (`TargetId`, `FrameId`, `SessionId`, `BrowserContextId`) ficam em `crate::types`.

## Param structs

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoThingParams {
    pub required_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<bool>,
}
```

- Adicionar `Default` quando todos os campos obrigatórios tiverem defaults razoáveis (ex: todos opcionais).
- Documentar cada campo copiando a description do JSON.

## Return structs

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoThingReturn {
    pub result: String,
}
```

- Campos opcionais no retorno usam `#[serde(default)]` em vez de `skip_serializing_if`.

## Events

```rust
/// Fired when something happens.
///
/// CDP: `Foo.thingHappened`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThingHappenedEvent {
    pub data: String,
}
```

## Trait

```rust
pub trait FooCommands {
    /// Description copiada do JSON.
    ///
    /// CDP: `Foo.doThing`
    async fn foo_do_thing(&self, params: &DoThingParams) -> Result<DoThingReturn>;
}
```

- Sem params: `async fn foo_close(&self) -> Result<()>`
- Param simples (1-2 campos obrigatórios, 0 opcionais): pode usar args diretos em vez de struct.
- Sem return: `-> Result<()>`
- Return com um unico campo: ainda retorna a struct wrapper (consistencia).

## Impl for CdpSession

- Com retorno: `self.call("Foo.doThing", params).await`
- Sem retorno: `self.call_no_response("Foo.doThing", params).await`
- Sem params: `self.call("Foo.doThing", &serde_json::json!({})).await`
- Args diretos no trait: criar `InternalParams` struct privada na seção Impl.

## O que excluir

- Comandos marcados `"deprecated": true` no JSON.
- Tipos que existem apenas para suportar comandos deprecated.

## Type mapping

| JSON type | Rust type |
|---|---|
| `string` | `String` |
| `integer` | `i64` |
| `number` | `f64` |
| `boolean` | `bool` |
| `object` (sem properties) | `serde_json::Value` |
| `array` de `T` | `Vec<T>` |
| `$ref` local (ex: `Bounds`) | `Bounds` |
| `$ref` cross-domain (ex: `Target.TargetID`) | import de `crate::types` ou do outro modulo cdp |
| campo `optional: true` | `Option<T>` |

## Checklist para novo dominio

1. Ler o `{domain}.json` correspondente
2. Criar `{domain}.rs` seguindo a estrutura acima
3. Adicionar `pub mod {domain};` em `mod.rs`
4. Rodar `cargo check -p chromium-driver`
5. Verificar zero warnings
