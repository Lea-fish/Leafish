# Building Leafish for Android

This guide assumes a Linux host using glibc (most distributions).

## Install the Android NDK

You can either download the NDK directly, or use the Android SDK tools to manage your NDK's.
The latter has the benefit of being able to keep your NDK up-to-date relatively easy.

### Download the NDK directly

The NDK can be downloaded from [the Android developer site](https://developer.android.com/ndk/downloads/).
Download the Linux package and unzip it somewhere, we'll use `~/.android` as the target directory in this guide.

The eventual path will then be `~/.android/android-ndk-r<version>`.
To finish up, export that path as `$NDK_HOME`, so for example `export NDK_HOME=~/.android/android-ndk-r23`.

### Android SDK tools

Download either Android Studio or the Android command line tools from [here](https://developer.android.com/studio/#downloads).
Since you probably won't use the editor, this guide will use the command line tools.

Create a directory `~/.android/cmdline-tools` and unzip the downloaded archive to there.
Rename the unzip directory to `latest`, so the ending path will be `~/.android/cmdline-tools/latest` which includes the `bin` and `lib` directories.

Use the sdkmanager to install the `ndk-bundle`:

```sh
~/.android/cmdline-tools/latest/bin/sdkmanager install ndk-bundle
```

To finish up, export the installed NDK as `$NDK_HOME`, so for example `export NDK_HOME=~/.android/ndk-bundle/`.

### Configure Rust and Cargo

Using `rustup`, install the Android target for which you want to build:

```sh
rustup target add armv7-linux-android aarch64-linux-android i686-linux-android
```

Then we have to tell Cargo what cross-compiler to use depending on what platform we want to target.
For example for a x86 Android device:

```sh
export CC="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/i686-linux-android30-clang"
```

When building for `aarch64` replace `i686` for `aarch64`, and for `armv7` replace it for `armv7a`.

### Building

Invoke Cargo as usual but with the `--target=<target>-linux-android` argument added.
For example:

```sh
cargo build --target=i686-linux-android
```
