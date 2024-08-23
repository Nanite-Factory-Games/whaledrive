# WhaleDrive

This crate is a work in progress. Currently it downloads the compressed layers and
unpacks them for every image. Ideally it would decompress once and then reuse the decompressed
folders, but in practice this is very difficult to do and decompressing into the output folder
directly seems to be the best solution.

This utility also has to be run as root, because it needs to mount the image and write to it.

A simple cli utility to download docker images and create ext4 .img
files from them.

This utility outputs human readable JSON to stdout. This allows the
user to easily pipe the output to other tools like jq.

## Installation
You can install the utility with cargo:

```
cargo install whaledrive
```

## Usage
The whaledrive utility helps you manage container images efficiently with various commands and global options.

### Global Options
```sh
-b, --base-path <path>
```
Specify the folder where this utility will store data. The default is the data folder in the current working directory.


### Commands
<ul>

<li><b>info</b>: Get info about an image

```sh
cargo-whaledrive info <image> [--os <os>] [--architecture <arch>]
```
<ul>
    <li><b>image</b>: The name and optional tag of the image (e.g., ubuntu:20.04).</li>
    <li><b>--os</b>: The operating system the image is for (default: linux).</li>
    <li><b>--architecture</b>: The architecture the image is for (default: amd64).</li>
</ul>
</li><!-- End image info -->
<li><b>build</b>: Create an image from a registry

```sh
cargo-whaledrive build <image> [--os <os>] [--architecture <arch>]
```

<ul>
<li><b>image</b>: The name and optional tag of the image.</li>
<li><b>--os</b>: The operating system the image is for (default: linux).</li>
<li><b>--architecture</b>: The architecture the image is for (default: amd64).</li>
</ul>
</li><!-- End build image -->

<li><b>images</b>: List all images currently stored

```sh
cargo-whaledrive images [--os <os>] [--platform <platform>]
```
<ul>
<li><b>--os</b>: Filter by operating system.</li>
<li><b>--platform</b>: Filter by platform.</li>
</ul>
</li><!-- End list images -->


<li><b>rm</b>: Remove an image

```sh
cargo-whaledrive rm <image> [--prune] [--os <os>] [--architecture <arch>]
```
<ul>
    <li><b>image</b>: The name and optional tag of the image.</li>
    <li><b>--prune</b>: Also remove unreferenced layers associated with the image.</li>
    <li><b>--os</b>: Specify the operating system the image is for.</li>
    <li><b>--architecture</b>: Specify the architecture the image is for.</li>
</ul>
</li>
<li><b>prune</b>: Remove unreferenced images and layers

```sh
cargo-whaledrive prune
```
</li><!-- End prune -->
</ul><!-- End commands list -->



### Examples
Get info about an Ubuntu image:

```sh
cargo-whaledrive info ubuntu:20.04
```

Build an image for arm64:
```sh
cargo-whaledrive build myimage --architecture arm64
```
List images for a specific OS:

```sh
cargo-whaledrive images --os linux
```
Remove an image and clean up unused layers:

```sh
cargo-whaledrive rm myimage --prune
```