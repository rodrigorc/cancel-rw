# cancel-rw

Crate that provides a newtype that makes any synchronous `Read`, `Write` or `Seek` 
into a cancellable.

Its use case is to be able to cancel synchronous operation that are usually short lived,
and not `async`. Sometimes these sync operations may take quite some time, and the program
needs to be able to cancel it. If it were `async` that would be trivial, but being all sync
reads and writes, it must be run to completions.

Sometimes with `io`, you can just close the underlying socket or file and hope for a quick
error. But in the general case that is not so easy.
