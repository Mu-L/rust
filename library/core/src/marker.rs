//! Primitive traits and types representing basic properties of types.
//!
//! Rust types can be classified in various useful ways according to
//! their intrinsic properties. These classifications are represented
//! as traits.

#![stable(feature = "rust1", since = "1.0.0")]

mod variance;

#[unstable(feature = "phantom_variance_markers", issue = "135806")]
pub use self::variance::{
    PhantomContravariant, PhantomContravariantLifetime, PhantomCovariant, PhantomCovariantLifetime,
    PhantomInvariant, PhantomInvariantLifetime, Variance, variance,
};
use crate::cell::UnsafeCell;
use crate::cmp;
use crate::fmt::Debug;
use crate::hash::{Hash, Hasher};
use crate::pin::UnsafePinned;

// NOTE: for consistent error messages between `core` and `minicore`, all `diagnostic` attributes
// should be replicated exactly in `minicore` (if `minicore` defines the item).

/// Implements a given marker trait for multiple types at the same time.
///
/// The basic syntax looks like this:
/// ```ignore private macro
/// marker_impls! { MarkerTrait for u8, i8 }
/// ```
/// You can also implement `unsafe` traits
/// ```ignore private macro
/// marker_impls! { unsafe MarkerTrait for u8, i8 }
/// ```
/// Add attributes to all impls:
/// ```ignore private macro
/// marker_impls! {
///     #[allow(lint)]
///     #[unstable(feature = "marker_trait", issue = "none")]
///     MarkerTrait for u8, i8
/// }
/// ```
/// And use generics:
/// ```ignore private macro
/// marker_impls! {
///     MarkerTrait for
///         u8, i8,
///         {T: ?Sized} *const T,
///         {T: ?Sized} *mut T,
///         {T: MarkerTrait} PhantomData<T>,
///         u32,
/// }
/// ```
#[unstable(feature = "internal_impls_macro", issue = "none")]
// Allow implementations of `UnsizedConstParamTy` even though std cannot use that feature.
#[allow_internal_unstable(unsized_const_params)]
macro marker_impls {
    ( $(#[$($meta:tt)*])* $Trait:ident for $({$($bounds:tt)*})? $T:ty $(, $($rest:tt)*)? ) => {
        $(#[$($meta)*])* impl< $($($bounds)*)? > $Trait for $T {}
        marker_impls! { $(#[$($meta)*])* $Trait for $($($rest)*)? }
    },
    ( $(#[$($meta:tt)*])* $Trait:ident for ) => {},

    ( $(#[$($meta:tt)*])* unsafe $Trait:ident for $({$($bounds:tt)*})? $T:ty $(, $($rest:tt)*)? ) => {
        $(#[$($meta)*])* unsafe impl< $($($bounds)*)? > $Trait for $T {}
        marker_impls! { $(#[$($meta)*])* unsafe $Trait for $($($rest)*)? }
    },
    ( $(#[$($meta:tt)*])* unsafe $Trait:ident for ) => {},
}

/// Types that can be transferred across thread boundaries.
///
/// This trait is automatically implemented when the compiler determines it's
/// appropriate.
///
/// An example of a non-`Send` type is the reference-counting pointer
/// [`rc::Rc`][`Rc`]. If two threads attempt to clone [`Rc`]s that point to the same
/// reference-counted value, they might try to update the reference count at the
/// same time, which is [undefined behavior][ub] because [`Rc`] doesn't use atomic
/// operations. Its cousin [`sync::Arc`][arc] does use atomic operations (incurring
/// some overhead) and thus is `Send`.
///
/// See [the Nomicon](../../nomicon/send-and-sync.html) and the [`Sync`] trait for more details.
///
/// [`Rc`]: ../../std/rc/struct.Rc.html
/// [arc]: ../../std/sync/struct.Arc.html
/// [ub]: ../../reference/behavior-considered-undefined.html
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_diagnostic_item = "Send"]
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be sent between threads safely",
    label = "`{Self}` cannot be sent between threads safely"
)]
pub unsafe auto trait Send {
    // empty.
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> !Send for *const T {}
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> !Send for *mut T {}

// Most instances arise automatically, but this instance is needed to link up `T: Sync` with
// `&T: Send` (and it also removes the unsound default instance `T Send` -> `&T: Send` that would
// otherwise exist).
#[stable(feature = "rust1", since = "1.0.0")]
unsafe impl<T: Sync + PointeeSized> Send for &T {}

/// Types with a constant size known at compile time.
///
/// All type parameters have an implicit bound of `Sized`. The special syntax
/// `?Sized` can be used to remove this bound if it's not appropriate.
///
/// ```
/// # #![allow(dead_code)]
/// struct Foo<T>(T);
/// struct Bar<T: ?Sized>(T);
///
/// // struct FooUse(Foo<[i32]>); // error: Sized is not implemented for [i32]
/// struct BarUse(Bar<[i32]>); // OK
/// ```
///
/// The one exception is the implicit `Self` type of a trait. A trait does not
/// have an implicit `Sized` bound as this is incompatible with [trait object]s
/// where, by definition, the trait needs to work with all possible implementors,
/// and thus could be any size.
///
/// Although Rust will let you bind `Sized` to a trait, you won't
/// be able to use it to form a trait object later:
///
/// ```
/// # #![allow(unused_variables)]
/// trait Foo { }
/// trait Bar: Sized { }
///
/// struct Impl;
/// impl Foo for Impl { }
/// impl Bar for Impl { }
///
/// let x: &dyn Foo = &Impl;    // OK
/// // let y: &dyn Bar = &Impl; // error: the trait `Bar` cannot
///                             // be made into an object
/// ```
///
/// [trait object]: ../../book/ch17-02-trait-objects.html
#[doc(alias = "?", alias = "?Sized")]
#[stable(feature = "rust1", since = "1.0.0")]
#[lang = "sized"]
#[diagnostic::on_unimplemented(
    message = "the size for values of type `{Self}` cannot be known at compilation time",
    label = "doesn't have a size known at compile-time"
)]
#[fundamental] // for Default, for example, which requires that `[T]: !Default` be evaluatable
#[rustc_specialization_trait]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
// `Sized` being coinductive, despite having supertraits, is okay as there are no user-written impls,
// and we know that the supertraits are always implemented if the subtrait is just by looking at
// the builtin impls.
#[rustc_coinductive]
pub trait Sized: MetaSized {
    // Empty.
}

/// Types with a size that can be determined from pointer metadata.
#[unstable(feature = "sized_hierarchy", issue = "none")]
#[lang = "meta_sized"]
#[diagnostic::on_unimplemented(
    message = "the size for values of type `{Self}` cannot be known",
    label = "doesn't have a known size"
)]
#[fundamental]
#[rustc_specialization_trait]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
// `MetaSized` being coinductive, despite having supertraits, is okay for the same reasons as
// `Sized` above.
#[rustc_coinductive]
pub trait MetaSized: PointeeSized {
    // Empty
}

/// Types that may or may not have a size.
#[unstable(feature = "sized_hierarchy", issue = "none")]
#[lang = "pointee_sized"]
#[diagnostic::on_unimplemented(
    message = "values of type `{Self}` may or may not have a size",
    label = "may or may not have a known size"
)]
#[fundamental]
#[rustc_specialization_trait]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
#[rustc_coinductive]
pub trait PointeeSized {
    // Empty
}

/// Types that can be "unsized" to a dynamically-sized type.
///
/// For example, the sized array type `[i8; 2]` implements `Unsize<[i8]>` and
/// `Unsize<dyn fmt::Debug>`.
///
/// All implementations of `Unsize` are provided automatically by the compiler.
/// Those implementations are:
///
/// - Arrays `[T; N]` implement `Unsize<[T]>`.
/// - A type implements `Unsize<dyn Trait + 'a>` if all of these conditions are met:
///   - The type implements `Trait`.
///   - `Trait` is dyn-compatible[^1].
///   - The type is sized.
///   - The type outlives `'a`.
/// - Trait objects `dyn TraitA + AutoA... + 'a` implement `Unsize<dyn TraitB + AutoB... + 'b>`
///    if all of these conditions are met:
///   - `TraitB` is a supertrait of `TraitA`.
///   - `AutoB...` is a subset of `AutoA...`.
///   - `'a` outlives `'b`.
/// - Structs `Foo<..., T1, ..., Tn, ...>` implement `Unsize<Foo<..., U1, ..., Un, ...>>`
///   where any number of (type and const) parameters may be changed if all of these conditions
///   are met:
///   - Only the last field of `Foo` has a type involving the parameters `T1`, ..., `Tn`.
///   - All other parameters of the struct are equal.
///   - `Field<T1, ..., Tn>: Unsize<Field<U1, ..., Un>>`, where `Field<...>` stands for the actual
///     type of the struct's last field.
///
/// `Unsize` is used along with [`ops::CoerceUnsized`] to allow
/// "user-defined" containers such as [`Rc`] to contain dynamically-sized
/// types. See the [DST coercion RFC][RFC982] and [the nomicon entry on coercion][nomicon-coerce]
/// for more details.
///
/// [`ops::CoerceUnsized`]: crate::ops::CoerceUnsized
/// [`Rc`]: ../../std/rc/struct.Rc.html
/// [RFC982]: https://github.com/rust-lang/rfcs/blob/master/text/0982-dst-coercion.md
/// [nomicon-coerce]: ../../nomicon/coercions.html
/// [^1]: Formerly known as *object safe*.
#[unstable(feature = "unsize", issue = "18598")]
#[lang = "unsize"]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
pub trait Unsize<T: PointeeSized>: PointeeSized {
    // Empty.
}

/// Required trait for constants used in pattern matches.
///
/// Constants are only allowed as patterns if (a) their type implements
/// `PartialEq`, and (b) interpreting the value of the constant as a pattern
/// is equivalent to calling `PartialEq`. This ensures that constants used as
/// patterns cannot expose implementation details in an unexpected way or
/// cause semver hazards.
///
/// This trait ensures point (b).
/// Any type that derives `PartialEq` automatically implements this trait.
///
/// Implementing this trait (which is unstable) is a way for type authors to explicitly allow
/// comparing const values of this type; that operation will recursively compare all fields
/// (including private fields), even if that behavior differs from `PartialEq`. This can make it
/// semver-breaking to add further private fields to a type.
#[unstable(feature = "structural_match", issue = "31434")]
#[diagnostic::on_unimplemented(message = "the type `{Self}` does not `#[derive(PartialEq)]`")]
#[lang = "structural_peq"]
pub trait StructuralPartialEq {
    // Empty.
}

marker_impls! {
    #[unstable(feature = "structural_match", issue = "31434")]
    StructuralPartialEq for
        usize, u8, u16, u32, u64, u128,
        isize, i8, i16, i32, i64, i128,
        bool,
        char,
        str /* Technically requires `[u8]: StructuralPartialEq` */,
        (),
        {T, const N: usize} [T; N],
        {T} [T],
        {T: PointeeSized} &T,
}

/// Types whose values can be duplicated simply by copying bits.
///
/// By default, variable bindings have 'move semantics.' In other
/// words:
///
/// ```
/// #[derive(Debug)]
/// struct Foo;
///
/// let x = Foo;
///
/// let y = x;
///
/// // `x` has moved into `y`, and so cannot be used
///
/// // println!("{x:?}"); // error: use of moved value
/// ```
///
/// However, if a type implements `Copy`, it instead has 'copy semantics':
///
/// ```
/// // We can derive a `Copy` implementation. `Clone` is also required, as it's
/// // a supertrait of `Copy`.
/// #[derive(Debug, Copy, Clone)]
/// struct Foo;
///
/// let x = Foo;
///
/// let y = x;
///
/// // `y` is a copy of `x`
///
/// println!("{x:?}"); // A-OK!
/// ```
///
/// It's important to note that in these two examples, the only difference is whether you
/// are allowed to access `x` after the assignment. Under the hood, both a copy and a move
/// can result in bits being copied in memory, although this is sometimes optimized away.
///
/// ## How can I implement `Copy`?
///
/// There are two ways to implement `Copy` on your type. The simplest is to use `derive`:
///
/// ```
/// #[derive(Copy, Clone)]
/// struct MyStruct;
/// ```
///
/// You can also implement `Copy` and `Clone` manually:
///
/// ```
/// struct MyStruct;
///
/// impl Copy for MyStruct { }
///
/// impl Clone for MyStruct {
///     fn clone(&self) -> MyStruct {
///         *self
///     }
/// }
/// ```
///
/// There is a small difference between the two. The `derive` strategy will also place a `Copy`
/// bound on type parameters:
///
/// ```
/// #[derive(Clone)]
/// struct MyStruct<T>(T);
///
/// impl<T: Copy> Copy for MyStruct<T> { }
/// ```
///
/// This isn't always desired. For example, shared references (`&T`) can be copied regardless of
/// whether `T` is `Copy`. Likewise, a generic struct containing markers such as [`PhantomData`]
/// could potentially be duplicated with a bit-wise copy.
///
/// ## What's the difference between `Copy` and `Clone`?
///
/// Copies happen implicitly, for example as part of an assignment `y = x`. The behavior of
/// `Copy` is not overloadable; it is always a simple bit-wise copy.
///
/// Cloning is an explicit action, `x.clone()`. The implementation of [`Clone`] can
/// provide any type-specific behavior necessary to duplicate values safely. For example,
/// the implementation of [`Clone`] for [`String`] needs to copy the pointed-to string
/// buffer in the heap. A simple bitwise copy of [`String`] values would merely copy the
/// pointer, leading to a double free down the line. For this reason, [`String`] is [`Clone`]
/// but not `Copy`.
///
/// [`Clone`] is a supertrait of `Copy`, so everything which is `Copy` must also implement
/// [`Clone`]. If a type is `Copy` then its [`Clone`] implementation only needs to return `*self`
/// (see the example above).
///
/// ## When can my type be `Copy`?
///
/// A type can implement `Copy` if all of its components implement `Copy`. For example, this
/// struct can be `Copy`:
///
/// ```
/// # #[allow(dead_code)]
/// #[derive(Copy, Clone)]
/// struct Point {
///    x: i32,
///    y: i32,
/// }
/// ```
///
/// A struct can be `Copy`, and [`i32`] is `Copy`, therefore `Point` is eligible to be `Copy`.
/// By contrast, consider
///
/// ```
/// # #![allow(dead_code)]
/// # struct Point;
/// struct PointList {
///     points: Vec<Point>,
/// }
/// ```
///
/// The struct `PointList` cannot implement `Copy`, because [`Vec<T>`] is not `Copy`. If we
/// attempt to derive a `Copy` implementation, we'll get an error:
///
/// ```text
/// the trait `Copy` cannot be implemented for this type; field `points` does not implement `Copy`
/// ```
///
/// Shared references (`&T`) are also `Copy`, so a type can be `Copy`, even when it holds
/// shared references of types `T` that are *not* `Copy`. Consider the following struct,
/// which can implement `Copy`, because it only holds a *shared reference* to our non-`Copy`
/// type `PointList` from above:
///
/// ```
/// # #![allow(dead_code)]
/// # struct PointList;
/// #[derive(Copy, Clone)]
/// struct PointListWrapper<'a> {
///     point_list_ref: &'a PointList,
/// }
/// ```
///
/// ## When *can't* my type be `Copy`?
///
/// Some types can't be copied safely. For example, copying `&mut T` would create an aliased
/// mutable reference. Copying [`String`] would duplicate responsibility for managing the
/// [`String`]'s buffer, leading to a double free.
///
/// Generalizing the latter case, any type implementing [`Drop`] can't be `Copy`, because it's
/// managing some resource besides its own [`size_of::<T>`] bytes.
///
/// If you try to implement `Copy` on a struct or enum containing non-`Copy` data, you will get
/// the error [E0204].
///
/// [E0204]: ../../error_codes/E0204.html
///
/// ## When *should* my type be `Copy`?
///
/// Generally speaking, if your type _can_ implement `Copy`, it should. Keep in mind, though,
/// that implementing `Copy` is part of the public API of your type. If the type might become
/// non-`Copy` in the future, it could be prudent to omit the `Copy` implementation now, to
/// avoid a breaking API change.
///
/// ## Additional implementors
///
/// In addition to the [implementors listed below][impls],
/// the following types also implement `Copy`:
///
/// * Function item types (i.e., the distinct types defined for each function)
/// * Function pointer types (e.g., `fn() -> i32`)
/// * Closure types, if they capture no value from the environment
///   or if all such captured values implement `Copy` themselves.
///   Note that variables captured by shared reference always implement `Copy`
///   (even if the referent doesn't),
///   while variables captured by mutable reference never implement `Copy`.
///
/// [`Vec<T>`]: ../../std/vec/struct.Vec.html
/// [`String`]: ../../std/string/struct.String.html
/// [`size_of::<T>`]: size_of
/// [impls]: #implementors
#[stable(feature = "rust1", since = "1.0.0")]
#[lang = "copy"]
// FIXME(matthewjasper) This allows copying a type that doesn't implement
// `Copy` because of unsatisfied lifetime bounds (copying `A<'_>` when only
// `A<'static>: Copy` and `A<'_>: Clone`).
// We have this attribute here for now only because there are quite a few
// existing specializations on `Copy` that already exist in the standard
// library, and there's no way to safely have this behavior right now.
#[rustc_unsafe_specialization_marker]
#[rustc_diagnostic_item = "Copy"]
pub trait Copy: Clone {
    // Empty.
}

/// Derive macro generating an impl of the trait `Copy`.
#[rustc_builtin_macro]
#[stable(feature = "builtin_macro_prelude", since = "1.38.0")]
#[allow_internal_unstable(core_intrinsics, derive_clone_copy)]
pub macro Copy($item:item) {
    /* compiler built-in */
}

// Implementations of `Copy` for primitive types.
//
// Implementations that cannot be described in Rust
// are implemented in `traits::SelectionContext::copy_clone_conditions()`
// in `rustc_trait_selection`.
marker_impls! {
    #[stable(feature = "rust1", since = "1.0.0")]
    Copy for
        usize, u8, u16, u32, u64, u128,
        isize, i8, i16, i32, i64, i128,
        f16, f32, f64, f128,
        bool, char,
        {T: PointeeSized} *const T,
        {T: PointeeSized} *mut T,

}

#[unstable(feature = "never_type", issue = "35121")]
impl Copy for ! {}

/// Shared references can be copied, but mutable references *cannot*!
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> Copy for &T {}

/// Marker trait for the types that are allowed in union fields and unsafe
/// binder types.
///
/// Implemented for:
/// * `&T`, `&mut T` for all `T`,
/// * `ManuallyDrop<T>` for all `T`,
/// * tuples and arrays whose elements implement `BikeshedGuaranteedNoDrop`,
/// * or otherwise, all types that are `Copy`.
///
/// Notably, this doesn't include all trivially-destructible types for semver
/// reasons.
///
/// Bikeshed name for now. This trait does not do anything other than reflect the
/// set of types that are allowed within unions for field validity.
#[unstable(feature = "bikeshed_guaranteed_no_drop", issue = "none")]
#[lang = "bikeshed_guaranteed_no_drop"]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
#[doc(hidden)]
pub trait BikeshedGuaranteedNoDrop {}

/// Types for which it is safe to share references between threads.
///
/// This trait is automatically implemented when the compiler determines
/// it's appropriate.
///
/// The precise definition is: a type `T` is [`Sync`] if and only if `&T` is
/// [`Send`]. In other words, if there is no possibility of
/// [undefined behavior][ub] (including data races) when passing
/// `&T` references between threads.
///
/// As one would expect, primitive types like [`u8`] and [`f64`]
/// are all [`Sync`], and so are simple aggregate types containing them,
/// like tuples, structs and enums. More examples of basic [`Sync`]
/// types include "immutable" types like `&T`, and those with simple
/// inherited mutability, such as [`Box<T>`][box], [`Vec<T>`][vec] and
/// most other collection types. (Generic parameters need to be [`Sync`]
/// for their container to be [`Sync`].)
///
/// A somewhat surprising consequence of the definition is that `&mut T`
/// is `Sync` (if `T` is `Sync`) even though it seems like that might
/// provide unsynchronized mutation. The trick is that a mutable
/// reference behind a shared reference (that is, `& &mut T`)
/// becomes read-only, as if it were a `& &T`. Hence there is no risk
/// of a data race.
///
/// A shorter overview of how [`Sync`] and [`Send`] relate to referencing:
/// * `&T` is [`Send`] if and only if `T` is [`Sync`]
/// * `&mut T` is [`Send`] if and only if `T` is [`Send`]
/// * `&T` and `&mut T` are [`Sync`] if and only if `T` is [`Sync`]
///
/// Types that are not `Sync` are those that have "interior
/// mutability" in a non-thread-safe form, such as [`Cell`][cell]
/// and [`RefCell`][refcell]. These types allow for mutation of
/// their contents even through an immutable, shared reference. For
/// example the `set` method on [`Cell<T>`][cell] takes `&self`, so it requires
/// only a shared reference [`&Cell<T>`][cell]. The method performs no
/// synchronization, thus [`Cell`][cell] cannot be `Sync`.
///
/// Another example of a non-`Sync` type is the reference-counting
/// pointer [`Rc`][rc]. Given any reference [`&Rc<T>`][rc], you can clone
/// a new [`Rc<T>`][rc], modifying the reference counts in a non-atomic way.
///
/// For cases when one does need thread-safe interior mutability,
/// Rust provides [atomic data types], as well as explicit locking via
/// [`sync::Mutex`][mutex] and [`sync::RwLock`][rwlock]. These types
/// ensure that any mutation cannot cause data races, hence the types
/// are `Sync`. Likewise, [`sync::Arc`][arc] provides a thread-safe
/// analogue of [`Rc`][rc].
///
/// Any types with interior mutability must also use the
/// [`cell::UnsafeCell`][unsafecell] wrapper around the value(s) which
/// can be mutated through a shared reference. Failing to doing this is
/// [undefined behavior][ub]. For example, [`transmute`][transmute]-ing
/// from `&T` to `&mut T` is invalid.
///
/// See [the Nomicon][nomicon-send-and-sync] for more details about `Sync`.
///
/// [box]: ../../std/boxed/struct.Box.html
/// [vec]: ../../std/vec/struct.Vec.html
/// [cell]: crate::cell::Cell
/// [refcell]: crate::cell::RefCell
/// [rc]: ../../std/rc/struct.Rc.html
/// [arc]: ../../std/sync/struct.Arc.html
/// [atomic data types]: crate::sync::atomic
/// [mutex]: ../../std/sync/struct.Mutex.html
/// [rwlock]: ../../std/sync/struct.RwLock.html
/// [unsafecell]: crate::cell::UnsafeCell
/// [ub]: ../../reference/behavior-considered-undefined.html
/// [transmute]: crate::mem::transmute
/// [nomicon-send-and-sync]: ../../nomicon/send-and-sync.html
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_diagnostic_item = "Sync"]
#[lang = "sync"]
#[rustc_on_unimplemented(
    on(
        Self = "core::cell::once::OnceCell<T>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::OnceLock` instead"
    ),
    on(
        Self = "core::cell::Cell<u8>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU8` instead",
    ),
    on(
        Self = "core::cell::Cell<u16>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU16` instead",
    ),
    on(
        Self = "core::cell::Cell<u32>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU32` instead",
    ),
    on(
        Self = "core::cell::Cell<u64>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU64` instead",
    ),
    on(
        Self = "core::cell::Cell<usize>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicUsize` instead",
    ),
    on(
        Self = "core::cell::Cell<i8>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicI8` instead",
    ),
    on(
        Self = "core::cell::Cell<i16>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicI16` instead",
    ),
    on(
        Self = "core::cell::Cell<i32>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicI32` instead",
    ),
    on(
        Self = "core::cell::Cell<i64>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicI64` instead",
    ),
    on(
        Self = "core::cell::Cell<isize>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicIsize` instead",
    ),
    on(
        Self = "core::cell::Cell<bool>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicBool` instead",
    ),
    on(
        all(
            Self = "core::cell::Cell<T>",
            not(Self = "core::cell::Cell<u8>"),
            not(Self = "core::cell::Cell<u16>"),
            not(Self = "core::cell::Cell<u32>"),
            not(Self = "core::cell::Cell<u64>"),
            not(Self = "core::cell::Cell<usize>"),
            not(Self = "core::cell::Cell<i8>"),
            not(Self = "core::cell::Cell<i16>"),
            not(Self = "core::cell::Cell<i32>"),
            not(Self = "core::cell::Cell<i64>"),
            not(Self = "core::cell::Cell<isize>"),
            not(Self = "core::cell::Cell<bool>")
        ),
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock`",
    ),
    on(
        Self = "core::cell::RefCell<T>",
        note = "if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` instead",
    ),
    message = "`{Self}` cannot be shared between threads safely",
    label = "`{Self}` cannot be shared between threads safely"
)]
pub unsafe auto trait Sync {
    // FIXME(estebank): once support to add notes in `rustc_on_unimplemented`
    // lands in beta, and it has been extended to check whether a closure is
    // anywhere in the requirement chain, extend it as such (#48534):
    // ```
    // on(
    //     closure,
    //     note="`{Self}` cannot be shared safely, consider marking the closure `move`"
    // ),
    // ```

    // Empty
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> !Sync for *const T {}
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> !Sync for *mut T {}

/// Zero-sized type used to mark things that "act like" they own a `T`.
///
/// Adding a `PhantomData<T>` field to your type tells the compiler that your
/// type acts as though it stores a value of type `T`, even though it doesn't
/// really. This information is used when computing certain safety properties.
///
/// For a more in-depth explanation of how to use `PhantomData<T>`, please see
/// [the Nomicon](../../nomicon/phantom-data.html).
///
/// # A ghastly note 👻👻👻
///
/// Though they both have scary names, `PhantomData` and 'phantom types' are
/// related, but not identical. A phantom type parameter is simply a type
/// parameter which is never used. In Rust, this often causes the compiler to
/// complain, and the solution is to add a "dummy" use by way of `PhantomData`.
///
/// # Examples
///
/// ## Unused lifetime parameters
///
/// Perhaps the most common use case for `PhantomData` is a struct that has an
/// unused lifetime parameter, typically as part of some unsafe code. For
/// example, here is a struct `Slice` that has two pointers of type `*const T`,
/// presumably pointing into an array somewhere:
///
/// ```compile_fail,E0392
/// struct Slice<'a, T> {
///     start: *const T,
///     end: *const T,
/// }
/// ```
///
/// The intention is that the underlying data is only valid for the
/// lifetime `'a`, so `Slice` should not outlive `'a`. However, this
/// intent is not expressed in the code, since there are no uses of
/// the lifetime `'a` and hence it is not clear what data it applies
/// to. We can correct this by telling the compiler to act *as if* the
/// `Slice` struct contained a reference `&'a T`:
///
/// ```
/// use std::marker::PhantomData;
///
/// # #[allow(dead_code)]
/// struct Slice<'a, T> {
///     start: *const T,
///     end: *const T,
///     phantom: PhantomData<&'a T>,
/// }
/// ```
///
/// This also in turn infers the lifetime bound `T: 'a`, indicating
/// that any references in `T` are valid over the lifetime `'a`.
///
/// When initializing a `Slice` you simply provide the value
/// `PhantomData` for the field `phantom`:
///
/// ```
/// # #![allow(dead_code)]
/// # use std::marker::PhantomData;
/// # struct Slice<'a, T> {
/// #     start: *const T,
/// #     end: *const T,
/// #     phantom: PhantomData<&'a T>,
/// # }
/// fn borrow_vec<T>(vec: &Vec<T>) -> Slice<'_, T> {
///     let ptr = vec.as_ptr();
///     Slice {
///         start: ptr,
///         end: unsafe { ptr.add(vec.len()) },
///         phantom: PhantomData,
///     }
/// }
/// ```
///
/// ## Unused type parameters
///
/// It sometimes happens that you have unused type parameters which
/// indicate what type of data a struct is "tied" to, even though that
/// data is not actually found in the struct itself. Here is an
/// example where this arises with [FFI]. The foreign interface uses
/// handles of type `*mut ()` to refer to Rust values of different
/// types. We track the Rust type using a phantom type parameter on
/// the struct `ExternalResource` which wraps a handle.
///
/// [FFI]: ../../book/ch19-01-unsafe-rust.html#using-extern-functions-to-call-external-code
///
/// ```
/// # #![allow(dead_code)]
/// # trait ResType { }
/// # struct ParamType;
/// # mod foreign_lib {
/// #     pub fn new(_: usize) -> *mut () { 42 as *mut () }
/// #     pub fn do_stuff(_: *mut (), _: usize) {}
/// # }
/// # fn convert_params(_: ParamType) -> usize { 42 }
/// use std::marker::PhantomData;
///
/// struct ExternalResource<R> {
///    resource_handle: *mut (),
///    resource_type: PhantomData<R>,
/// }
///
/// impl<R: ResType> ExternalResource<R> {
///     fn new() -> Self {
///         let size_of_res = size_of::<R>();
///         Self {
///             resource_handle: foreign_lib::new(size_of_res),
///             resource_type: PhantomData,
///         }
///     }
///
///     fn do_stuff(&self, param: ParamType) {
///         let foreign_params = convert_params(param);
///         foreign_lib::do_stuff(self.resource_handle, foreign_params);
///     }
/// }
/// ```
///
/// ## Ownership and the drop check
///
/// The exact interaction of `PhantomData` with drop check **may change in the future**.
///
/// Currently, adding a field of type `PhantomData<T>` indicates that your type *owns* data of type
/// `T` in very rare circumstances. This in turn has effects on the Rust compiler's [drop check]
/// analysis. For the exact rules, see the [drop check] documentation.
///
/// ## Layout
///
/// For all `T`, the following are guaranteed:
/// * `size_of::<PhantomData<T>>() == 0`
/// * `align_of::<PhantomData<T>>() == 1`
///
/// [drop check]: Drop#drop-check
#[lang = "phantom_data"]
#[stable(feature = "rust1", since = "1.0.0")]
pub struct PhantomData<T: PointeeSized>;

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> Hash for PhantomData<T> {
    #[inline]
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> cmp::PartialEq for PhantomData<T> {
    fn eq(&self, _other: &PhantomData<T>) -> bool {
        true
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> cmp::Eq for PhantomData<T> {}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> cmp::PartialOrd for PhantomData<T> {
    fn partial_cmp(&self, _other: &PhantomData<T>) -> Option<cmp::Ordering> {
        Option::Some(cmp::Ordering::Equal)
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> cmp::Ord for PhantomData<T> {
    fn cmp(&self, _other: &PhantomData<T>) -> cmp::Ordering {
        cmp::Ordering::Equal
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> Copy for PhantomData<T> {}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: PointeeSized> Clone for PhantomData<T> {
    fn clone(&self) -> Self {
        Self
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_default", issue = "143894")]
impl<T: PointeeSized> const Default for PhantomData<T> {
    fn default() -> Self {
        Self
    }
}

#[unstable(feature = "structural_match", issue = "31434")]
impl<T: PointeeSized> StructuralPartialEq for PhantomData<T> {}

/// Compiler-internal trait used to indicate the type of enum discriminants.
///
/// This trait is automatically implemented for every type and does not add any
/// guarantees to [`mem::Discriminant`]. It is **undefined behavior** to transmute
/// between `DiscriminantKind::Discriminant` and `mem::Discriminant`.
///
/// [`mem::Discriminant`]: crate::mem::Discriminant
#[unstable(
    feature = "discriminant_kind",
    issue = "none",
    reason = "this trait is unlikely to ever be stabilized, use `mem::discriminant` instead"
)]
#[lang = "discriminant_kind"]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
pub trait DiscriminantKind {
    /// The type of the discriminant, which must satisfy the trait
    /// bounds required by `mem::Discriminant`.
    #[lang = "discriminant_type"]
    type Discriminant: Clone + Copy + Debug + Eq + PartialEq + Hash + Send + Sync + Unpin;
}

/// Used to determine whether a type contains
/// any `UnsafeCell` internally, but not through an indirection.
/// This affects, for example, whether a `static` of that type is
/// placed in read-only static memory or writable static memory.
/// This can be used to declare that a constant with a generic type
/// will not contain interior mutability, and subsequently allow
/// placing the constant behind references.
///
/// # Safety
///
/// This trait is a core part of the language, it is just expressed as a trait in libcore for
/// convenience. Do *not* implement it for other types.
// FIXME: Eventually this trait should become `#[rustc_deny_explicit_impl]`.
// That requires porting the impls below to native internal impls.
#[lang = "freeze"]
#[unstable(feature = "freeze", issue = "121675")]
pub unsafe auto trait Freeze {}

#[unstable(feature = "freeze", issue = "121675")]
impl<T: PointeeSized> !Freeze for UnsafeCell<T> {}
marker_impls! {
    #[unstable(feature = "freeze", issue = "121675")]
    unsafe Freeze for
        {T: PointeeSized} PhantomData<T>,
        {T: PointeeSized} *const T,
        {T: PointeeSized} *mut T,
        {T: PointeeSized} &T,
        {T: PointeeSized} &mut T,
}

/// Used to determine whether a type contains any `UnsafePinned` (or `PhantomPinned`) internally,
/// but not through an indirection. This affects, for example, whether we emit `noalias` metadata
/// for `&mut T` or not.
///
/// This is part of [RFC 3467](https://rust-lang.github.io/rfcs/3467-unsafe-pinned.html), and is
/// tracked by [#125735](https://github.com/rust-lang/rust/issues/125735).
#[lang = "unsafe_unpin"]
pub(crate) unsafe auto trait UnsafeUnpin {}

impl<T: ?Sized> !UnsafeUnpin for UnsafePinned<T> {}
unsafe impl<T: ?Sized> UnsafeUnpin for PhantomData<T> {}
unsafe impl<T: ?Sized> UnsafeUnpin for *const T {}
unsafe impl<T: ?Sized> UnsafeUnpin for *mut T {}
unsafe impl<T: ?Sized> UnsafeUnpin for &T {}
unsafe impl<T: ?Sized> UnsafeUnpin for &mut T {}

/// Types that do not require any pinning guarantees.
///
/// For information on what "pinning" is, see the [`pin` module] documentation.
///
/// Implementing the `Unpin` trait for `T` expresses the fact that `T` is pinning-agnostic:
/// it shall not expose nor rely on any pinning guarantees. This, in turn, means that a
/// `Pin`-wrapped pointer to such a type can feature a *fully unrestricted* API.
/// In other words, if `T: Unpin`, a value of type `T` will *not* be bound by the invariants
/// which pinning otherwise offers, even when "pinned" by a [`Pin<Ptr>`] pointing at it.
/// When a value of type `T` is pointed at by a [`Pin<Ptr>`], [`Pin`] will not restrict access
/// to the pointee value like it normally would, thus allowing the user to do anything that they
/// normally could with a non-[`Pin`]-wrapped `Ptr` to that value.
///
/// The idea of this trait is to alleviate the reduced ergonomics of APIs that require the use
/// of [`Pin`] for soundness for some types, but which also want to be used by other types that
/// don't care about pinning. The prime example of such an API is [`Future::poll`]. There are many
/// [`Future`] types that don't care about pinning. These futures can implement `Unpin` and
/// therefore get around the pinning related restrictions in the API, while still allowing the
/// subset of [`Future`]s which *do* require pinning to be implemented soundly.
///
/// For more discussion on the consequences of [`Unpin`] within the wider scope of the pinning
/// system, see the [section about `Unpin`] in the [`pin` module].
///
/// `Unpin` has no consequence at all for non-pinned data. In particular, [`mem::replace`] happily
/// moves `!Unpin` data, which would be immovable when pinned ([`mem::replace`] works for any
/// `&mut T`, not just when `T: Unpin`).
///
/// *However*, you cannot use [`mem::replace`] on `!Unpin` data which is *pinned* by being wrapped
/// inside a [`Pin<Ptr>`] pointing at it. This is because you cannot (safely) use a
/// [`Pin<Ptr>`] to get a `&mut T` to its pointee value, which you would need to call
/// [`mem::replace`], and *that* is what makes this system work.
///
/// So this, for example, can only be done on types implementing `Unpin`:
///
/// ```rust
/// # #![allow(unused_must_use)]
/// use std::mem;
/// use std::pin::Pin;
///
/// let mut string = "this".to_string();
/// let mut pinned_string = Pin::new(&mut string);
///
/// // We need a mutable reference to call `mem::replace`.
/// // We can obtain such a reference by (implicitly) invoking `Pin::deref_mut`,
/// // but that is only possible because `String` implements `Unpin`.
/// mem::replace(&mut *pinned_string, "other".to_string());
/// ```
///
/// This trait is automatically implemented for almost every type. The compiler is free
/// to take the conservative stance of marking types as [`Unpin`] so long as all of the types that
/// compose its fields are also [`Unpin`]. This is because if a type implements [`Unpin`], then it
/// is unsound for that type's implementation to rely on pinning-related guarantees for soundness,
/// *even* when viewed through a "pinning" pointer! It is the responsibility of the implementor of
/// a type that relies upon pinning for soundness to ensure that type is *not* marked as [`Unpin`]
/// by adding [`PhantomPinned`] field. For more details, see the [`pin` module] docs.
///
/// [`mem::replace`]: crate::mem::replace "mem replace"
/// [`Future`]: crate::future::Future "Future"
/// [`Future::poll`]: crate::future::Future::poll "Future poll"
/// [`Pin`]: crate::pin::Pin "Pin"
/// [`Pin<Ptr>`]: crate::pin::Pin "Pin"
/// [`pin` module]: crate::pin "pin module"
/// [section about `Unpin`]: crate::pin#unpin "pin module docs about unpin"
/// [`unsafe`]: ../../std/keyword.unsafe.html "keyword unsafe"
#[stable(feature = "pin", since = "1.33.0")]
#[diagnostic::on_unimplemented(
    note = "consider using the `pin!` macro\nconsider using `Box::pin` if you need to access the pinned value outside of the current scope",
    message = "`{Self}` cannot be unpinned"
)]
#[lang = "unpin"]
pub auto trait Unpin {}

/// A marker type which does not implement `Unpin`.
///
/// If a type contains a `PhantomPinned`, it will not implement `Unpin` by default.
//
// FIXME(unsafe_pinned): This is *not* a stable guarantee we want to make, at least not yet.
// Note that for backwards compatibility with the new [`UnsafePinned`] wrapper type, placing this
// marker in your struct acts as if you wrapped the entire struct in an `UnsafePinned`. This type
// will likely eventually be deprecated, and all new code should be using `UnsafePinned` instead.
#[stable(feature = "pin", since = "1.33.0")]
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PhantomPinned;

#[stable(feature = "pin", since = "1.33.0")]
impl !Unpin for PhantomPinned {}

// This is a small hack to allow existing code which uses PhantomPinned to opt-out of noalias to
// continue working. Ideally PhantomPinned could just wrap an `UnsafePinned<()>` to get the same
// effect, but we can't add a new field to an already stable unit struct -- that would be a breaking
// change.
impl !UnsafeUnpin for PhantomPinned {}

marker_impls! {
    #[stable(feature = "pin", since = "1.33.0")]
    Unpin for
        {T: PointeeSized} &T,
        {T: PointeeSized} &mut T,
}

marker_impls! {
    #[stable(feature = "pin_raw", since = "1.38.0")]
    Unpin for
        {T: PointeeSized} *const T,
        {T: PointeeSized} *mut T,
}

/// A marker for types that can be dropped.
///
/// This should be used for `~const` bounds,
/// as non-const bounds will always hold for every type.
#[unstable(feature = "const_destruct", issue = "133214")]
#[rustc_const_unstable(feature = "const_destruct", issue = "133214")]
#[lang = "destruct"]
#[rustc_on_unimplemented(message = "can't drop `{Self}`", append_const_msg)]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
#[const_trait]
pub trait Destruct {}

/// A marker for tuple types.
///
/// The implementation of this trait is built-in and cannot be implemented
/// for any user type.
#[unstable(feature = "tuple_trait", issue = "none")]
#[lang = "tuple_trait"]
#[diagnostic::on_unimplemented(message = "`{Self}` is not a tuple")]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
pub trait Tuple {}

/// A marker for types which can be used as types of `const` generic parameters.
///
/// These types must have a proper equivalence relation (`Eq`) and it must be automatically
/// derived (`StructuralPartialEq`). There's a hard-coded check in the compiler ensuring
/// that all fields are also `ConstParamTy`, which implies that recursively, all fields
/// are `StructuralPartialEq`.
#[lang = "const_param_ty"]
#[unstable(feature = "unsized_const_params", issue = "95174")]
#[diagnostic::on_unimplemented(message = "`{Self}` can't be used as a const parameter type")]
#[allow(multiple_supertrait_upcastable)]
// We name this differently than the derive macro so that the `adt_const_params` can
// be used independently of `unsized_const_params` without requiring a full path
// to the derive macro every time it is used. This should be renamed on stabilization.
pub trait ConstParamTy_: UnsizedConstParamTy + StructuralPartialEq + Eq {}

/// Derive macro generating an impl of the trait `ConstParamTy`.
#[rustc_builtin_macro]
#[allow_internal_unstable(unsized_const_params)]
#[unstable(feature = "adt_const_params", issue = "95174")]
pub macro ConstParamTy($item:item) {
    /* compiler built-in */
}

#[lang = "unsized_const_param_ty"]
#[unstable(feature = "unsized_const_params", issue = "95174")]
#[diagnostic::on_unimplemented(message = "`{Self}` can't be used as a const parameter type")]
/// A marker for types which can be used as types of `const` generic parameters.
///
/// Equivalent to [`ConstParamTy_`] except that this is used by
/// the `unsized_const_params` to allow for fake unstable impls.
pub trait UnsizedConstParamTy: StructuralPartialEq + Eq {}

/// Derive macro generating an impl of the trait `ConstParamTy`.
#[rustc_builtin_macro]
#[allow_internal_unstable(unsized_const_params)]
#[unstable(feature = "unsized_const_params", issue = "95174")]
pub macro UnsizedConstParamTy($item:item) {
    /* compiler built-in */
}

// FIXME(adt_const_params): handle `ty::FnDef`/`ty::Closure`
marker_impls! {
    #[unstable(feature = "adt_const_params", issue = "95174")]
    ConstParamTy_ for
        usize, u8, u16, u32, u64, u128,
        isize, i8, i16, i32, i64, i128,
        bool,
        char,
        (),
        {T: ConstParamTy_, const N: usize} [T; N],
}

marker_impls! {
    #[unstable(feature = "unsized_const_params", issue = "95174")]
    UnsizedConstParamTy for
        usize, u8, u16, u32, u64, u128,
        isize, i8, i16, i32, i64, i128,
        bool,
        char,
        (),
        {T: UnsizedConstParamTy, const N: usize} [T; N],

        str,
        {T: UnsizedConstParamTy} [T],
        {T: UnsizedConstParamTy + ?Sized} &T,
}

/// A common trait implemented by all function pointers.
//
// Note that while the trait is internal and unstable it is nevertheless
// exposed as a public bound of the stable `core::ptr::fn_addr_eq` function.
#[unstable(
    feature = "fn_ptr_trait",
    issue = "none",
    reason = "internal trait for implementing various traits for all function pointers"
)]
#[lang = "fn_ptr_trait"]
#[rustc_deny_explicit_impl]
#[rustc_do_not_implement_via_object]
pub trait FnPtr: Copy + Clone {
    /// Returns the address of the function pointer.
    #[lang = "fn_ptr_addr"]
    fn addr(self) -> *const ();
}

/// Derive macro that makes a smart pointer usable with trait objects.
///
/// # What this macro does
///
/// This macro is intended to be used with user-defined pointer types, and makes it possible to
/// perform coercions on the pointee of the user-defined pointer. There are two aspects to this:
///
/// ## Unsizing coercions of the pointee
///
/// By using the macro, the following example will compile:
/// ```
/// #![feature(derive_coerce_pointee)]
/// use std::marker::CoercePointee;
/// use std::ops::Deref;
///
/// #[derive(CoercePointee)]
/// #[repr(transparent)]
/// struct MySmartPointer<T: ?Sized>(Box<T>);
///
/// impl<T: ?Sized> Deref for MySmartPointer<T> {
///     type Target = T;
///     fn deref(&self) -> &T {
///         &self.0
///     }
/// }
///
/// trait MyTrait {}
///
/// impl MyTrait for i32 {}
///
/// fn main() {
///     let ptr: MySmartPointer<i32> = MySmartPointer(Box::new(4));
///
///     // This coercion would be an error without the derive.
///     let ptr: MySmartPointer<dyn MyTrait> = ptr;
/// }
/// ```
/// Without the `#[derive(CoercePointee)]` macro, this example would fail with the following error:
/// ```text
/// error[E0308]: mismatched types
///   --> src/main.rs:11:44
///    |
/// 11 |     let ptr: MySmartPointer<dyn MyTrait> = ptr;
///    |              ---------------------------   ^^^ expected `MySmartPointer<dyn MyTrait>`, found `MySmartPointer<i32>`
///    |              |
///    |              expected due to this
///    |
///    = note: expected struct `MySmartPointer<dyn MyTrait>`
///               found struct `MySmartPointer<i32>`
///    = help: `i32` implements `MyTrait` so you could box the found value and coerce it to the trait object `Box<dyn MyTrait>`, you will have to change the expected type as well
/// ```
///
/// ## Dyn compatibility
///
/// This macro allows you to dispatch on the user-defined pointer type. That is, traits using the
/// type as a receiver are dyn-compatible. For example, this compiles:
///
/// ```
/// #![feature(arbitrary_self_types, derive_coerce_pointee)]
/// use std::marker::CoercePointee;
/// use std::ops::Deref;
///
/// #[derive(CoercePointee)]
/// #[repr(transparent)]
/// struct MySmartPointer<T: ?Sized>(Box<T>);
///
/// impl<T: ?Sized> Deref for MySmartPointer<T> {
///     type Target = T;
///     fn deref(&self) -> &T {
///         &self.0
///     }
/// }
///
/// // You can always define this trait. (as long as you have #![feature(arbitrary_self_types)])
/// trait MyTrait {
///     fn func(self: MySmartPointer<Self>);
/// }
///
/// // But using `dyn MyTrait` requires #[derive(CoercePointee)].
/// fn call_func(value: MySmartPointer<dyn MyTrait>) {
///     value.func();
/// }
/// ```
/// If you remove the `#[derive(CoercePointee)]` annotation from the struct, then the above example
/// will fail with this error message:
/// ```text
/// error[E0038]: the trait `MyTrait` is not dyn compatible
///   --> src/lib.rs:21:36
///    |
/// 17 |     fn func(self: MySmartPointer<Self>);
///    |                   -------------------- help: consider changing method `func`'s `self` parameter to be `&self`: `&Self`
/// ...
/// 21 | fn call_func(value: MySmartPointer<dyn MyTrait>) {
///    |                                    ^^^^^^^^^^^ `MyTrait` is not dyn compatible
///    |
/// note: for a trait to be dyn compatible it needs to allow building a vtable
///       for more information, visit <https://doc.rust-lang.org/reference/items/traits.html#object-safety>
///   --> src/lib.rs:17:19
///    |
/// 16 | trait MyTrait {
///    |       ------- this trait is not dyn compatible...
/// 17 |     fn func(self: MySmartPointer<Self>);
///    |                   ^^^^^^^^^^^^^^^^^^^^ ...because method `func`'s `self` parameter cannot be dispatched on
/// ```
///
/// # Requirements for using the macro
///
/// This macro can only be used if:
/// * The type is a `#[repr(transparent)]` struct.
/// * The type of its non-zero-sized field must either be a standard library pointer type
///   (reference, raw pointer, `NonNull`, `Box`, `Rc`, `Arc`, etc.) or another user-defined type
///   also using the `#[derive(CoercePointee)]` macro.
/// * Zero-sized fields must not mention any generic parameters unless the zero-sized field has
///   type [`PhantomData`].
///
/// ## Multiple type parameters
///
/// If the type has multiple type parameters, then you must explicitly specify which one should be
/// used for dynamic dispatch. For example:
/// ```
/// # #![feature(derive_coerce_pointee)]
/// # use std::marker::{CoercePointee, PhantomData};
/// #[derive(CoercePointee)]
/// #[repr(transparent)]
/// struct MySmartPointer<#[pointee] T: ?Sized, U> {
///     ptr: Box<T>,
///     _phantom: PhantomData<U>,
/// }
/// ```
/// Specifying `#[pointee]` when the struct has only one type parameter is allowed, but not required.
///
/// # Examples
///
/// A custom implementation of the `Rc` type:
/// ```
/// #![feature(derive_coerce_pointee)]
/// use std::marker::CoercePointee;
/// use std::ops::Deref;
/// use std::ptr::NonNull;
///
/// #[derive(CoercePointee)]
/// #[repr(transparent)]
/// pub struct Rc<T: ?Sized> {
///     inner: NonNull<RcInner<T>>,
/// }
///
/// struct RcInner<T: ?Sized> {
///     refcount: usize,
///     value: T,
/// }
///
/// impl<T: ?Sized> Deref for Rc<T> {
///     type Target = T;
///     fn deref(&self) -> &T {
///         let ptr = self.inner.as_ptr();
///         unsafe { &(*ptr).value }
///     }
/// }
///
/// impl<T> Rc<T> {
///     pub fn new(value: T) -> Self {
///         let inner = Box::new(RcInner {
///             refcount: 1,
///             value,
///         });
///         Self {
///             inner: NonNull::from(Box::leak(inner)),
///         }
///     }
/// }
///
/// impl<T: ?Sized> Clone for Rc<T> {
///     fn clone(&self) -> Self {
///         // A real implementation would handle overflow here.
///         unsafe { (*self.inner.as_ptr()).refcount += 1 };
///         Self { inner: self.inner }
///     }
/// }
///
/// impl<T: ?Sized> Drop for Rc<T> {
///     fn drop(&mut self) {
///         let ptr = self.inner.as_ptr();
///         unsafe { (*ptr).refcount -= 1 };
///         if unsafe { (*ptr).refcount } == 0 {
///             drop(unsafe { Box::from_raw(ptr) });
///         }
///     }
/// }
/// ```
#[rustc_builtin_macro(CoercePointee, attributes(pointee))]
#[allow_internal_unstable(dispatch_from_dyn, coerce_unsized, unsize, coerce_pointee_validated)]
#[rustc_diagnostic_item = "CoercePointee"]
#[unstable(feature = "derive_coerce_pointee", issue = "123430")]
pub macro CoercePointee($item:item) {
    /* compiler built-in */
}

/// A trait that is implemented for ADTs with `derive(CoercePointee)` so that
/// the compiler can enforce the derive impls are valid post-expansion, since
/// the derive has stricter requirements than if the impls were written by hand.
///
/// This trait is not intended to be implemented by users or used other than
/// validation, so it should never be stabilized.
#[lang = "coerce_pointee_validated"]
#[unstable(feature = "coerce_pointee_validated", issue = "none")]
#[doc(hidden)]
pub trait CoercePointeeValidated {
    /* compiler built-in */
}
