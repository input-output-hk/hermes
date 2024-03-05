# Hermes Application Signatures

Signatures in Hermes Applications are created by Authors of the application.

If there is an independent Publisher/s of the application they too can attach a signature to the application.

Signatures are generated on a hash of the Applications contents from the root.

HDF5 filesystem is internally hierarchical.  
What this means is that it is possible to get the entire contents of a group as binary data.
This significantly simplifies hashing of the filesystem contents in a controlled way,
without needing to traverse the filesystem tree.

Recall the Package Filesystem hierarchy:

![Diagram](images/application_package_hierarchy.d2)

We can generate a complete hash of the entire Application with the following procedure:

```rust
  let mut hasher = Hash::new();
    
  // Update the hasher with each of the binary buffers
  hasher.update(bytes_of(`metadata.json`));

  if exists(`/srv`) {
    hasher.update(bytes_of(`/srv`));
  }

  if exists(`/usr`) {
    `hasher.update(bytes_of(`/usr`));
  }

  if exists(`/lib`) {
    hasher.update(bytes_of(`/lib`));
  }

  let digest = hasher.finalize()
```

In addition the following checks are made:

1. Has the bare minimum to be a viable application:
    * Is there a `/srv/www` group with at least 1 file in it; OR
    * Is there a `/lib` group with at least 1 validly signed Module in it.
  
2. Does the Application ONLY contain the groups and files as described in the Hierarchy diagram.

If either of these fails, the package is Invalid and can not be signed.
These checks are also made when any Application is loaded, to ensure it has not been tampered with.

Once the hash of the root groups is known, and the structure is validated,
it is a simple matter of generating or validating a signature from the Hash.

This method protects any Application from being tampered with once released by the Author,
and also allows it to be safely co-signed by a Publisher.
