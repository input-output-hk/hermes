//! HTTP host implementation for WASM runtime.

use crate::{
    runtime_extensions::bindings::wasi::{
        http::{
            self,
            outgoing_handler::{
                ErrorCode, FutureIncomingResponse, OutgoingRequest, RequestOptions,
            },
            types::{
                Duration, FieldKey, FieldValue, Fields, FutureTrailers, HeaderError, Headers,
                HostIncomingResponse, HostOutgoingResponse, IncomingBody, IncomingRequest,
                IncomingResponse, IoError, Method, OutgoingBody, OutgoingResponse,
                ResponseOutparam, Scheme, StatusCode, Trailers,
            },
        },
        io::streams::{InputStream, OutputStream},
    },
    state::HermesState,
};

impl http::types::HostFutureIncomingResponse for HermesState {
    /// Returns the incoming HTTP Response, or an error, once one is ready.
    ///
    /// The outer `option` represents future readiness. Users can wait on this
    /// `option` to become `some` using the `subscribe` method.
    ///
    /// The outer `result` is used to retrieve the response or error at most
    /// once. It will be success on the first call in which the outer option
    /// is `some`, and error on subsequent calls.
    ///
    /// The inner `result` represents that either the incoming HTTP Response
    /// status and headers have received successfully, or that an error
    /// occurred. Errors may also occur while consuming the response body,
    /// but those will be reported by the `incoming-body` and its
    /// `output-stream` child.
    fn get(
        &mut self, _res: wasmtime::component::Resource<FutureIncomingResponse>,
    ) -> wasmtime::Result<
        Option<Result<Result<wasmtime::component::Resource<IncomingResponse>, ErrorCode>, ()>>,
    > {
        todo!()
    }

    fn drop(
        &mut self, _rep: wasmtime::component::Resource<FutureIncomingResponse>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostFields for HermesState {
    /// Construct an empty HTTP Fields.
    ///
    /// The resulting `fields` is mutable.
    fn new(&mut self) -> wasmtime::Result<wasmtime::component::Resource<Fields>> {
        todo!()
    }

    /// Construct an HTTP Fields.
    ///
    /// The resulting `fields` is mutable.
    ///
    /// The list represents each key-value pair in the Fields. Keys
    /// which have multiple values are represented by multiple entries in this
    /// list with the same key.
    ///
    /// The tuple is a pair of the field key, represented as a string, and
    /// Value, represented as a list of bytes. In a valid Fields, all keys
    /// and values are valid UTF-8 strings. However, values are not always
    /// well-formed, so they are represented as a raw list of bytes.
    ///
    /// An error result will be returned if any header or value was
    /// syntactically invalid, or if a header was forbidden.
    fn from_list(
        &mut self, _entries: Vec<(FieldKey, FieldValue)>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Fields>, HeaderError>> {
        todo!()
    }

    /// Get all of the values corresponding to a key. If the key is not present
    /// in this `fields`, an empty list is returned. However, if the key is
    /// present but empty, this is represented by a list with one or more
    /// empty field-values present.
    fn get(
        &mut self, _fields: wasmtime::component::Resource<Fields>, _name: FieldKey,
    ) -> wasmtime::Result<Vec<FieldValue>> {
        todo!()
    }

    /// Returns `true` when the key is present in this `fields`. If the key is
    /// syntactically invalid, `false` is returned.
    fn has(
        &mut self, _fields: wasmtime::component::Resource<Fields>, _name: FieldKey,
    ) -> wasmtime::Result<bool> {
        todo!()
    }

    /// Set all of the values for a key. Clears any existing values for that
    /// key, if they have been set.
    ///
    /// Fails with `header-error.immutable` if the `fields` are immutable.
    fn set(
        &mut self, _fields: wasmtime::component::Resource<Fields>, _name: FieldKey,
        _value: Vec<FieldValue>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        todo!()
    }

    /// Delete all values for a key. Does nothing if no values for the key
    /// exist.
    ///
    /// Fails with `header-error.immutable` if the `fields` are immutable.
    fn delete(
        &mut self, _fields: wasmtime::component::Resource<Fields>, _name: FieldKey,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        todo!()
    }

    /// Append a value for a key. Does not change or delete any existing
    /// values for that key.
    ///
    /// Fails with `header-error.immutable` if the `fields` are immutable.
    fn append(
        &mut self, _fields: wasmtime::component::Resource<Fields>, _name: FieldKey,
        _value: FieldValue,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        todo!()
    }

    /// Retrieve the full set of keys and values in the Fields. Like the
    /// constructor, the list represents each key-value pair.
    ///
    /// The outer list represents each key-value pair in the Fields. Keys
    /// which have multiple values are represented by multiple entries in this
    /// list with the same key.
    fn entries(
        &mut self, _fields: wasmtime::component::Resource<Fields>,
    ) -> wasmtime::Result<Vec<(FieldKey, FieldValue)>> {
        todo!()
    }

    /// Make a deep copy of the Fields. Equivalent in behavior to calling the
    /// `fields` constructor on the return value of `entries`. The resulting
    /// `fields` is mutable.
    fn clone(
        &mut self, _fields: wasmtime::component::Resource<Fields>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Fields>> {
        todo!()
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<Fields>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostFutureTrailers for HermesState {
    /// Returns the contents of the trailers, or an error which occurred,
    /// once the future is ready.
    ///
    /// The outer `option` represents future readiness. Users can wait on this
    /// `option` to become `some` using the `subscribe` method.
    ///
    /// The outer `result` is used to retrieve the trailers or error at most
    /// once. It will be success on the first call in which the outer option
    /// is `some`, and error on subsequent calls.
    ///
    /// The inner `result` represents that either the HTTP Request or Response
    /// body, as well as any trailers, were received successfully, or that an
    /// error occurred receiving them. The optional `trailers` indicates whether
    /// or not trailers were present in the body.
    ///
    /// When some `trailers` are returned by this method, the `trailers`
    /// resource is immutable, and a child. Use of the `set`, `append`, or
    /// `delete` methods will return an error, and the resource must be
    /// dropped before the parent `future-trailers` is dropped.
    fn get(
        &mut self, _rep: wasmtime::component::Resource<FutureTrailers>,
    ) -> wasmtime::Result<
        Option<Result<Result<Option<wasmtime::component::Resource<Trailers>>, ErrorCode>, ()>>,
    > {
        todo!()
    }

    fn drop(
        &mut self, _rep: wasmtime::component::Resource<FutureTrailers>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostOutgoingBody for HermesState {
    /// Returns a stream for writing the body contents.
    ///
    /// The returned `output-stream` is a child resource: it must be dropped
    /// before the parent `outgoing-body` resource is dropped (or finished),
    /// otherwise the `outgoing-body` drop or `finish` will trap.
    ///
    /// Returns success on the first call: the `output-stream` resource for
    /// this `outgoing-body` may be retrieved at most once. Subsequent calls
    /// will return error.
    fn write(
        &mut self, _rep: wasmtime::component::Resource<OutgoingBody>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutputStream>, ()>> {
        todo!()
    }

    /// Finalize an outgoing body, optionally providing trailers. This must be
    /// called to signal that the response is complete. If the `outgoing-body`
    /// is dropped without calling `outgoing-body.finalize`, the implementation
    /// should treat the body as corrupted.
    ///
    /// Fails if the body\'s `outgoing-request` or `outgoing-response` was
    /// constructed with a Content-Length header, and the contents written
    /// to the body (via `write`) does not match the value given in the
    /// Content-Length.
    fn finish(
        &mut self, _this: wasmtime::component::Resource<OutgoingBody>,
        _trailers: Option<wasmtime::component::Resource<Trailers>>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<OutgoingBody>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostOutgoingResponse for HermesState {
    /// Construct an `outgoing-response`, with a default `status-code` of `200`.
    /// If a different `status-code` is needed, it must be set via the
    /// `set-status-code` method.
    ///
    /// * `headers` is the HTTP Headers for the Response.
    fn new(
        &mut self, _headers: wasmtime::component::Resource<Headers>,
    ) -> wasmtime::Result<wasmtime::component::Resource<OutgoingResponse>> {
        todo!()
    }

    /// Get the HTTP Status Code for the Response.
    fn status_code(
        &mut self, _rep: wasmtime::component::Resource<OutgoingResponse>,
    ) -> wasmtime::Result<StatusCode> {
        todo!()
    }

    /// Set the HTTP Status Code for the Response. Fails if the status-code
    /// given is not a valid http status code.
    fn set_status_code(
        &mut self, _rep: wasmtime::component::Resource<OutgoingResponse>, _status_code: StatusCode,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    /// Get the headers associated with the Request.
    ///
    /// The returned `headers` resource is immutable: `set`, `append`, and
    /// `delete` operations will fail with `header-error.immutable`.
    ///
    /// This headers resource is a child: it must be dropped before the parent
    /// `outgoing-request` is dropped, or its ownership is transferred to
    /// another component by e.g. `outgoing-handler.handle`.
    fn headers(
        &mut self, _rep: wasmtime::component::Resource<OutgoingResponse>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        todo!()
    }

    /// Returns the resource corresponding to the outgoing Body for this Response.
    ///
    /// Returns success on the first call: the `outgoing-body` resource for
    /// this `outgoing-response` can be retrieved at most once. Subsequent
    /// calls will return error.
    fn body(
        &mut self, _rep: wasmtime::component::Resource<OutgoingResponse>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutgoingBody>, ()>> {
        todo!()
    }

    fn drop(
        &mut self, _rep: wasmtime::component::Resource<OutgoingResponse>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostIncomingBody for HermesState {
    /// Returns the contents of the body, as a stream of bytes.
    ///
    /// Returns success on first call: the stream representing the contents
    /// can be retrieved at most once. Subsequent calls will return error.
    ///
    /// The returned `input-stream` resource is a child: it must be dropped
    /// before the parent `incoming-body` is dropped, or consumed by
    /// `incoming-body.finish`.
    ///
    /// This invariant ensures that the implementation can determine whether
    /// the user is consuming the contents of the body, waiting on the
    /// `future-trailers` to be ready, or neither. This allows for network
    /// backpressure is to be applied when the user is consuming the body,
    /// and for that backpressure to not inhibit delivery of the trailers if
    /// the user does not read the entire body.
    fn stream(
        &mut self, _rep: wasmtime::component::Resource<IncomingBody>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<InputStream>, ()>> {
        todo!()
    }

    /// Takes ownership of `incoming-body`, and returns a `future-trailers`.
    /// This function will trap if the `input-stream` child is still alive.
    fn finish(
        &mut self, _this: wasmtime::component::Resource<IncomingBody>,
    ) -> wasmtime::Result<wasmtime::component::Resource<FutureTrailers>> {
        todo!()
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<IncomingBody>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostIncomingResponse for HermesState {
    /// Returns the status code from the incoming response.
    fn status(
        &mut self, _rep: wasmtime::component::Resource<IncomingResponse>,
    ) -> wasmtime::Result<StatusCode> {
        todo!()
    }

    /// Returns the headers from the incoming response.
    ///
    /// The returned `headers` resource is immutable: `set`, `append`, and
    /// `delete` operations will fail with `header-error.immutable`.
    ///
    /// This headers resource is a child: it must be dropped before the parent
    /// `incoming-response` is dropped.
    fn headers(
        &mut self, _rep: wasmtime::component::Resource<IncomingResponse>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        todo!()
    }

    /// Returns the incoming body. May be called at most once. Returns error
    /// if called additional times.
    fn consume(
        &mut self, _rep: wasmtime::component::Resource<IncomingResponse>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<IncomingBody>, ()>> {
        todo!()
    }

    fn drop(
        &mut self, _rep: wasmtime::component::Resource<IncomingResponse>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostResponseOutparam for HermesState {
    /// Set the value of the `response-outparam` to either send a response,
    /// or indicate an error.
    ///
    /// This method consumes the `response-outparam` to ensure that it is
    /// called at most once. If it is never called, the implementation
    /// will respond with an error.
    ///
    /// The user may provide an `error` to `response` to allow the
    /// implementation determine how to respond with an HTTP error response.
    fn set(
        &mut self, _param: wasmtime::component::Resource<ResponseOutparam>,
        _response: Result<wasmtime::component::Resource<OutgoingResponse>, ErrorCode>,
    ) -> wasmtime::Result<()> {
        todo!()
    }

    fn drop(
        &mut self, _rep: wasmtime::component::Resource<ResponseOutparam>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostRequestOptions for HermesState {
    /// Construct a default `request-options` value.
    fn new(&mut self) -> wasmtime::Result<wasmtime::component::Resource<RequestOptions>> {
        todo!()
    }

    /// The timeout for the initial connect to the HTTP Server.
    fn connect_timeout(
        &mut self, _rep: wasmtime::component::Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        todo!()
    }

    /// Set the timeout for the initial connect to the HTTP Server. An error
    /// return value indicates that this timeout is not supported.
    fn set_connect_timeout(
        &mut self, _rep: wasmtime::component::Resource<RequestOptions>, _duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    /// The timeout for receiving the first byte of the Response body.
    fn first_byte_timeout(
        &mut self, _rep: wasmtime::component::Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        todo!()
    }

    /// Set the timeout for receiving the first byte of the Response body. An
    /// error return value indicates that this timeout is not supported.
    fn set_first_byte_timeout(
        &mut self, _rep: wasmtime::component::Resource<RequestOptions>, _duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    /// The timeout for receiving subsequent chunks of bytes in the Response
    /// body stream.
    fn between_bytes_timeout(
        &mut self, _rep: wasmtime::component::Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        todo!()
    }

    /// Set the timeout for receiving subsequent chunks of bytes in the Response
    /// body stream. An error return value indicates that this timeout is not
    /// supported.
    fn set_between_bytes_timeout(
        &mut self, _rep: wasmtime::component::Resource<RequestOptions>, _duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    fn drop(
        &mut self, _rep: wasmtime::component::Resource<RequestOptions>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostOutgoingRequest for HermesState {
    /// Construct a new `outgoing-request` with a default `method` of `GET`, and
    /// `none` values for `path-with-query`, `scheme`, and `authority`.
    ///
    /// * `headers` is the HTTP Headers for the Request.
    ///
    /// It is possible to construct, or manipulate with the accessor functions
    /// below, an `outgoing-request` with an invalid combination of `scheme`
    /// and `authority`, or `headers` which are not permitted to be sent.
    /// It is the obligation of the `outgoing-handler.handle` implementation
    /// to reject invalid constructions of `outgoing-request`.
    fn new(
        &mut self, _headers: wasmtime::component::Resource<Headers>,
    ) -> wasmtime::Result<wasmtime::component::Resource<OutgoingRequest>> {
        todo!()
    }

    /// Returns the resource corresponding to the outgoing Body for this
    /// Request.
    ///
    /// Returns success on the first call: the `outgoing-body` resource for
    /// this `outgoing-request` can be retrieved at most once. Subsequent
    /// calls will return error.
    fn body(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutgoingBody>, ()>> {
        todo!()
    }

    /// Get the Method for the Request.
    fn method(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Method> {
        todo!()
    }

    /// Set the Method for the Request. Fails if the string present in a
    /// `method.other` argument is not a syntactically valid method.
    fn set_method(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>, _method: Method,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    /// Get the combination of the HTTP Path and Query for the Request.
    /// When `none`, this represents an empty Path and empty Query.
    fn path_with_query(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    /// Set the combination of the HTTP Path and Query for the Request.
    /// When `none`, this represents an empty Path and empty Query. Fails is the
    /// string given is not a syntactically valid path and query uri component.
    fn set_path_with_query(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>,
        _path_with_query: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    /// Get the HTTP Related Scheme for the Request. When `none`, the
    /// implementation may choose an appropriate default scheme.
    fn scheme(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<Scheme>> {
        todo!()
    }

    /// Set the HTTP Related Scheme for the Request. When `none`, the
    /// implementation may choose an appropriate default scheme. Fails if the
    /// string given is not a syntactically valid uri scheme.
    fn set_scheme(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>, _scheme: Option<Scheme>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    /// Get the HTTP Authority for the Request. A value of `none` may be used
    /// with Related Schemes which do not require an Authority. The HTTP and
    /// HTTPS schemes always require an authority.
    fn authority(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    /// Set the HTTP Authority for the Request. A value of `none` may be used
    /// with Related Schemes which do not require an Authority. The HTTP and
    /// HTTPS schemes always require an authority. Fails if the string given is
    /// not a syntactically valid uri authority.
    fn set_authority(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>, _authority: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    /// Get the headers associated with the Request.
    ///
    /// The returned `headers` resource is immutable: `set`, `append`, and
    /// `delete` operations will fail with `header-error.immutable`.
    ///
    /// This headers resource is a child: it must be dropped before the parent
    /// `outgoing-request` is dropped, or its ownership is transferred to
    /// another component by e.g. `outgoing-handler.handle`.
    fn headers(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        todo!()
    }

    fn drop(
        &mut self, _rep: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostIncomingRequest for HermesState {
    /// Returns the method of the incoming request.
    fn method(
        &mut self, _rep: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Method> {
        todo!()
    }

    /// Returns the path with query parameters from the request, as a string.
    fn path_with_query(
        &mut self, _rep: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    /// Returns the protocol scheme from the request.
    fn scheme(
        &mut self, _rep: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<Scheme>> {
        todo!()
    }

    /// Returns the authority from the request, if it was present.
    fn authority(
        &mut self, _rep: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    /// Get the `headers` associated with the request.
    ///
    /// The returned `headers` resource is immutable: `set`, `append`, and
    /// `delete` operations will fail with `header-error.immutable`.
    ///
    /// The `headers` returned are a child resource: it must be dropped before
    /// the parent `incoming-request` is dropped. Dropping this
    /// `incoming-request` before all children are dropped will trap.
    fn headers(
        &mut self, _rep: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        todo!()
    }

    /// Gives the `incoming-body` associated with this request. Will only
    /// return success at most once, and subsequent calls will return error.
    fn consume(
        &mut self, _rep: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<IncomingBody>, ()>> {
        todo!()
    }

    fn drop(
        &mut self, _rep: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::Host for HermesState {
    /// Attempts to extract a http-related `error` from the wasi:io `error`
    /// provided.
    ///
    /// Stream operations which return
    /// `wasi:io/stream/stream-error::last-operation-failed` have a payload of
    /// type `wasi:io/error/error` with more information about the operation
    /// that failed. This payload can be passed through to this function to see
    /// if there\'s http-related information about the error to return.
    ///
    /// Note that this function is fallible because not all io-errors are
    /// http-related errors.
    fn http_error_code(
        &mut self, _err: wasmtime::component::Resource<IoError>,
    ) -> wasmtime::Result<Option<ErrorCode>> {
        todo!()
    }
}

impl http::outgoing_handler::Host for HermesState {
    /// This function is invoked with an outgoing HTTP Request, and it returns
    /// a resource `future-incoming-response` which represents an HTTP Response
    /// which may arrive in the future.
    ///
    /// The `options` argument accepts optional parameters for the HTTP
    /// protocol\'s transport layer.
    ///
    /// This function may return an error if the `outgoing-request` is invalid
    /// or not allowed to be made. Otherwise, protocol errors are reported
    /// through the `future-incoming-response`.
    fn handle(
        &mut self, _request: wasmtime::component::Resource<OutgoingRequest>,
        _options: Option<wasmtime::component::Resource<RequestOptions>>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<FutureIncomingResponse>, ErrorCode>>
    {
        todo!()
    }
}
