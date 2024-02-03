//! Host - WASI - HTTP implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    wasi::{
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
    HermesState, NewState,
};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}

impl http::types::HostFutureIncomingResponse for HermesState {
    #[doc = " *"]
    #[doc = "     /// Returns a pollable which becomes ready when either the Response has"]
    #[doc = "     /// been received, or an error has occurred. When this pollable is ready,"]
    #[doc = "     /// the `get` method will return `some`."]
    #[doc = "     subscribe: func() -> pollable;"]
    #[doc = "     */"]
    #[doc = " Returns the incoming HTTP Response, or an error, once one is ready."]
    #[doc = " "]
    #[doc = " The outer `option` represents future readiness. Users can wait on this"]
    #[doc = " `option` to become `some` using the `subscribe` method."]
    #[doc = " "]
    #[doc = " The outer `result` is used to retrieve the response or error at most"]
    #[doc = " once. It will be success on the first call in which the outer option"]
    #[doc = " is `some`, and error on subsequent calls."]
    #[doc = " "]
    #[doc = " The inner `result` represents that either the incoming HTTP Response"]
    #[doc = " status and headers have received successfully, or that an error"]
    #[doc = " occurred. Errors may also occur while consuming the response body,"]
    #[doc = " but those will be reported by the `incoming-body` and its"]
    #[doc = " `output-stream` child."]
    fn get(
        &mut self, self_: wasmtime::component::Resource<FutureIncomingResponse>,
    ) -> wasmtime::Result<
        Option<Result<Result<wasmtime::component::Resource<IncomingResponse>, ErrorCode>, ()>>,
    > {
        todo!()
    }

    fn drop(
        &mut self, rep: wasmtime::component::Resource<FutureIncomingResponse>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostFields for HermesState {
    #[doc = " Construct an empty HTTP Fields."]
    #[doc = " "]
    #[doc = " The resulting `fields` is mutable."]
    fn new(&mut self) -> wasmtime::Result<wasmtime::component::Resource<Fields>> {
        todo!()
    }

    #[doc = " Construct an HTTP Fields."]
    #[doc = " "]
    #[doc = " The resulting `fields` is mutable."]
    #[doc = " "]
    #[doc = " The list represents each key-value pair in the Fields. Keys"]
    #[doc = " which have multiple values are represented by multiple entries in this"]
    #[doc = " list with the same key."]
    #[doc = " "]
    #[doc = " The tuple is a pair of the field key, represented as a string, and"]
    #[doc = " Value, represented as a list of bytes. In a valid Fields, all keys"]
    #[doc = " and values are valid UTF-8 strings. However, values are not always"]
    #[doc = " well-formed, so they are represented as a raw list of bytes."]
    #[doc = " "]
    #[doc = " An error result will be returned if any header or value was"]
    #[doc = " syntactically invalid, or if a header was forbidden."]
    fn from_list(
        &mut self, entries: Vec<(FieldKey, FieldValue)>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Fields>, HeaderError>> {
        todo!()
    }

    #[doc = " Get all of the values corresponding to a key. If the key is not present"]
    #[doc = " in this `fields`, an empty list is returned. However, if the key is"]
    #[doc = " present but empty, this is represented by a list with one or more"]
    #[doc = " empty field-values present."]
    fn get(
        &mut self, self_: wasmtime::component::Resource<Fields>, name: FieldKey,
    ) -> wasmtime::Result<Vec<FieldValue>> {
        todo!()
    }

    #[doc = " Returns `true` when the key is present in this `fields`. If the key is"]
    #[doc = " syntactically invalid, `false` is returned."]
    fn has(
        &mut self, self_: wasmtime::component::Resource<Fields>, name: FieldKey,
    ) -> wasmtime::Result<bool> {
        todo!()
    }

    #[doc = " Set all of the values for a key. Clears any existing values for that"]
    #[doc = " key, if they have been set."]
    #[doc = " "]
    #[doc = " Fails with `header-error.immutable` if the `fields` are immutable."]
    fn set(
        &mut self, self_: wasmtime::component::Resource<Fields>, name: FieldKey,
        value: Vec<FieldValue>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        todo!()
    }

    #[doc = " Delete all values for a key. Does nothing if no values for the key"]
    #[doc = " exist."]
    #[doc = " "]
    #[doc = " Fails with `header-error.immutable` if the `fields` are immutable."]
    fn delete(
        &mut self, self_: wasmtime::component::Resource<Fields>, name: FieldKey,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        todo!()
    }

    #[doc = " Append a value for a key. Does not change or delete any existing"]
    #[doc = " values for that key."]
    #[doc = " "]
    #[doc = " Fails with `header-error.immutable` if the `fields` are immutable."]
    fn append(
        &mut self, self_: wasmtime::component::Resource<Fields>, name: FieldKey, value: FieldValue,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        todo!()
    }

    #[doc = " Retrieve the full set of keys and values in the Fields. Like the"]
    #[doc = " constructor, the list represents each key-value pair."]
    #[doc = " "]
    #[doc = " The outer list represents each key-value pair in the Fields. Keys"]
    #[doc = " which have multiple values are represented by multiple entries in this"]
    #[doc = " list with the same key."]
    fn entries(
        &mut self, self_: wasmtime::component::Resource<Fields>,
    ) -> wasmtime::Result<Vec<(FieldKey, FieldValue)>> {
        todo!()
    }

    #[doc = " Make a deep copy of the Fields. Equivalent in behavior to calling the"]
    #[doc = " `fields` constructor on the return value of `entries`. The resulting"]
    #[doc = " `fields` is mutable."]
    fn clone(
        &mut self, self_: wasmtime::component::Resource<Fields>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Fields>> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Fields>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostFutureTrailers for HermesState {
    #[doc = " Returns a pollable which becomes ready when either the trailers have"]
    #[doc = " been received, or an error has occurred. When this pollable is ready,"]
    #[doc = " the `get` method will return `some`."]
    #[doc = " subscribe: func() -> pollable; // Hermes does NOT support `poll`"]
    #[doc = " Returns the contents of the trailers, or an error which occurred,"]
    #[doc = " once the future is ready."]
    #[doc = " "]
    #[doc = " The outer `option` represents future readiness. Users can wait on this"]
    #[doc = " `option` to become `some` using the `subscribe` method."]
    #[doc = " "]
    #[doc = " The outer `result` is used to retrieve the trailers or error at most"]
    #[doc = " once. It will be success on the first call in which the outer option"]
    #[doc = " is `some`, and error on subsequent calls."]
    #[doc = " "]
    #[doc = " The inner `result` represents that either the HTTP Request or Response"]
    #[doc = " body, as well as any trailers, were received successfully, or that an"]
    #[doc = " error occurred receiving them. The optional `trailers` indicates whether"]
    #[doc = " or not trailers were present in the body."]
    #[doc = " "]
    #[doc = " When some `trailers` are returned by this method, the `trailers`"]
    #[doc = " resource is immutable, and a child. Use of the `set`, `append`, or"]
    #[doc = " `delete` methods will return an error, and the resource must be"]
    #[doc = " dropped before the parent `future-trailers` is dropped."]
    fn get(
        &mut self, self_: wasmtime::component::Resource<FutureTrailers>,
    ) -> wasmtime::Result<
        Option<Result<Result<Option<wasmtime::component::Resource<Trailers>>, ErrorCode>, ()>>,
    > {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<FutureTrailers>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostOutgoingBody for HermesState {
    #[doc = " Returns a stream for writing the body contents."]
    #[doc = " "]
    #[doc = " The returned `output-stream` is a child resource: it must be dropped"]
    #[doc = " before the parent `outgoing-body` resource is dropped (or finished),"]
    #[doc = " otherwise the `outgoing-body` drop or `finish` will trap."]
    #[doc = " "]
    #[doc = " Returns success on the first call: the `output-stream` resource for"]
    #[doc = " this `outgoing-body` may be retrieved at most once. Subsequent calls"]
    #[doc = " will return error."]
    fn write(
        &mut self, self_: wasmtime::component::Resource<OutgoingBody>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutputStream>, ()>> {
        todo!()
    }

    #[doc = " Finalize an outgoing body, optionally providing trailers. This must be"]
    #[doc = " called to signal that the response is complete. If the `outgoing-body`"]
    #[doc = " is dropped without calling `outgoing-body.finalize`, the implementation"]
    #[doc = " should treat the body as corrupted."]
    #[doc = " "]
    #[doc = " Fails if the body\\'s `outgoing-request` or `outgoing-response` was"]
    #[doc = " constructed with a Content-Length header, and the contents written"]
    #[doc = " to the body (via `write`) does not match the value given in the"]
    #[doc = " Content-Length."]
    fn finish(
        &mut self, this: wasmtime::component::Resource<OutgoingBody>,
        trailers: Option<wasmtime::component::Resource<Trailers>>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<OutgoingBody>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostOutgoingResponse for HermesState {
    #[doc = " Construct an `outgoing-response`, with a default `status-code` of `200`."]
    #[doc = " If a different `status-code` is needed, it must be set via the"]
    #[doc = " `set-status-code` method."]
    #[doc = " "]
    #[doc = " * `headers` is the HTTP Headers for the Response."]
    fn new(
        &mut self, headers: wasmtime::component::Resource<Headers>,
    ) -> wasmtime::Result<wasmtime::component::Resource<OutgoingResponse>> {
        todo!()
    }

    #[doc = " Get the HTTP Status Code for the Response."]
    fn status_code(
        &mut self, self_: wasmtime::component::Resource<OutgoingResponse>,
    ) -> wasmtime::Result<StatusCode> {
        todo!()
    }

    #[doc = " Set the HTTP Status Code for the Response. Fails if the status-code"]
    #[doc = " given is not a valid http status code."]
    fn set_status_code(
        &mut self, self_: wasmtime::component::Resource<OutgoingResponse>, status_code: StatusCode,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    #[doc = " Get the headers associated with the Request."]
    #[doc = " "]
    #[doc = " The returned `headers` resource is immutable: `set`, `append`, and"]
    #[doc = " `delete` operations will fail with `header-error.immutable`."]
    #[doc = " "]
    #[doc = " This headers resource is a child: it must be dropped before the parent"]
    #[doc = " `outgoing-request` is dropped, or its ownership is transferred to"]
    #[doc = " another component by e.g. `outgoing-handler.handle`."]
    fn headers(
        &mut self, self_: wasmtime::component::Resource<OutgoingResponse>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        todo!()
    }

    #[doc = " Returns the resource corresponding to the outgoing Body for this Response."]
    #[doc = " "]
    #[doc = " Returns success on the first call: the `outgoing-body` resource for"]
    #[doc = " this `outgoing-response` can be retrieved at most once. Subsequent"]
    #[doc = " calls will return error."]
    fn body(
        &mut self, self_: wasmtime::component::Resource<OutgoingResponse>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutgoingBody>, ()>> {
        todo!()
    }

    fn drop(
        &mut self, rep: wasmtime::component::Resource<OutgoingResponse>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostIncomingBody for HermesState {
    #[doc = " Returns the contents of the body, as a stream of bytes."]
    #[doc = " "]
    #[doc = " Returns success on first call: the stream representing the contents"]
    #[doc = " can be retrieved at most once. Subsequent calls will return error."]
    #[doc = " "]
    #[doc = " The returned `input-stream` resource is a child: it must be dropped"]
    #[doc = " before the parent `incoming-body` is dropped, or consumed by"]
    #[doc = " `incoming-body.finish`."]
    #[doc = " "]
    #[doc = " This invariant ensures that the implementation can determine whether"]
    #[doc = " the user is consuming the contents of the body, waiting on the"]
    #[doc = " `future-trailers` to be ready, or neither. This allows for network"]
    #[doc = " backpressure is to be applied when the user is consuming the body,"]
    #[doc = " and for that backpressure to not inhibit delivery of the trailers if"]
    #[doc = " the user does not read the entire body."]
    fn stream(
        &mut self, self_: wasmtime::component::Resource<IncomingBody>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<InputStream>, ()>> {
        todo!()
    }

    #[doc = " Takes ownership of `incoming-body`, and returns a `future-trailers`."]
    #[doc = " This function will trap if the `input-stream` child is still alive."]
    fn finish(
        &mut self, this: wasmtime::component::Resource<IncomingBody>,
    ) -> wasmtime::Result<wasmtime::component::Resource<FutureTrailers>> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<IncomingBody>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostIncomingResponse for HermesState {
    #[doc = " Returns the status code from the incoming response."]
    fn status(
        &mut self, self_: wasmtime::component::Resource<IncomingResponse>,
    ) -> wasmtime::Result<StatusCode> {
        todo!()
    }

    #[doc = " Returns the headers from the incoming response."]
    #[doc = " "]
    #[doc = " The returned `headers` resource is immutable: `set`, `append`, and"]
    #[doc = " `delete` operations will fail with `header-error.immutable`."]
    #[doc = " "]
    #[doc = " This headers resource is a child: it must be dropped before the parent"]
    #[doc = " `incoming-response` is dropped."]
    fn headers(
        &mut self, self_: wasmtime::component::Resource<IncomingResponse>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        todo!()
    }

    #[doc = " Returns the incoming body. May be called at most once. Returns error"]
    #[doc = " if called additional times."]
    fn consume(
        &mut self, self_: wasmtime::component::Resource<IncomingResponse>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<IncomingBody>, ()>> {
        todo!()
    }

    fn drop(
        &mut self, rep: wasmtime::component::Resource<IncomingResponse>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostResponseOutparam for HermesState {
    #[doc = " Set the value of the `response-outparam` to either send a response,"]
    #[doc = " or indicate an error."]
    #[doc = " "]
    #[doc = " This method consumes the `response-outparam` to ensure that it is"]
    #[doc = " called at most once. If it is never called, the implementation"]
    #[doc = " will respond with an error."]
    #[doc = " "]
    #[doc = " The user may provide an `error` to `response` to allow the"]
    #[doc = " implementation determine how to respond with an HTTP error response."]
    fn set(
        &mut self, param: wasmtime::component::Resource<ResponseOutparam>,
        response: Result<wasmtime::component::Resource<OutgoingResponse>, ErrorCode>,
    ) -> wasmtime::Result<()> {
        todo!()
    }

    fn drop(
        &mut self, rep: wasmtime::component::Resource<ResponseOutparam>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostRequestOptions for HermesState {
    #[doc = " Construct a default `request-options` value."]
    fn new(&mut self) -> wasmtime::Result<wasmtime::component::Resource<RequestOptions>> {
        todo!()
    }

    #[doc = " The timeout for the initial connect to the HTTP Server."]
    fn connect_timeout(
        &mut self, self_: wasmtime::component::Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        todo!()
    }

    #[doc = " Set the timeout for the initial connect to the HTTP Server. An error"]
    #[doc = " return value indicates that this timeout is not supported."]
    fn set_connect_timeout(
        &mut self, self_: wasmtime::component::Resource<RequestOptions>, duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    #[doc = " The timeout for receiving the first byte of the Response body."]
    fn first_byte_timeout(
        &mut self, self_: wasmtime::component::Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        todo!()
    }

    #[doc = " Set the timeout for receiving the first byte of the Response body. An"]
    #[doc = " error return value indicates that this timeout is not supported."]
    fn set_first_byte_timeout(
        &mut self, self_: wasmtime::component::Resource<RequestOptions>, duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    #[doc = " The timeout for receiving subsequent chunks of bytes in the Response"]
    #[doc = " body stream."]
    fn between_bytes_timeout(
        &mut self, self_: wasmtime::component::Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        todo!()
    }

    #[doc = " Set the timeout for receiving subsequent chunks of bytes in the Response"]
    #[doc = " body stream. An error return value indicates that this timeout is not"]
    #[doc = " supported."]
    fn set_between_bytes_timeout(
        &mut self, self_: wasmtime::component::Resource<RequestOptions>, duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<RequestOptions>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostOutgoingRequest for HermesState {
    #[doc = " Construct a new `outgoing-request` with a default `method` of `GET`, and"]
    #[doc = " `none` values for `path-with-query`, `scheme`, and `authority`."]
    #[doc = " "]
    #[doc = " * `headers` is the HTTP Headers for the Request."]
    #[doc = " "]
    #[doc = " It is possible to construct, or manipulate with the accessor functions"]
    #[doc = " below, an `outgoing-request` with an invalid combination of `scheme`"]
    #[doc = " and `authority`, or `headers` which are not permitted to be sent."]
    #[doc = " It is the obligation of the `outgoing-handler.handle` implementation"]
    #[doc = " to reject invalid constructions of `outgoing-request`."]
    fn new(
        &mut self, headers: wasmtime::component::Resource<Headers>,
    ) -> wasmtime::Result<wasmtime::component::Resource<OutgoingRequest>> {
        todo!()
    }

    #[doc = " Returns the resource corresponding to the outgoing Body for this"]
    #[doc = " Request."]
    #[doc = " "]
    #[doc = " Returns success on the first call: the `outgoing-body` resource for"]
    #[doc = " this `outgoing-request` can be retrieved at most once. Subsequent"]
    #[doc = " calls will return error."]
    fn body(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutgoingBody>, ()>> {
        todo!()
    }

    #[doc = " Get the Method for the Request."]
    fn method(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Method> {
        todo!()
    }

    #[doc = " Set the Method for the Request. Fails if the string present in a"]
    #[doc = " `method.other` argument is not a syntactically valid method."]
    fn set_method(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>, method: Method,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    #[doc = " Get the combination of the HTTP Path and Query for the Request."]
    #[doc = " When `none`, this represents an empty Path and empty Query."]
    fn path_with_query(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    #[doc = " Set the combination of the HTTP Path and Query for the Request."]
    #[doc = " When `none`, this represents an empty Path and empty Query. Fails is the"]
    #[doc = " string given is not a syntactically valid path and query uri component."]
    fn set_path_with_query(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>,
        path_with_query: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    #[doc = " Get the HTTP Related Scheme for the Request. When `none`, the"]
    #[doc = " implementation may choose an appropriate default scheme."]
    fn scheme(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<Scheme>> {
        todo!()
    }

    #[doc = " Set the HTTP Related Scheme for the Request. When `none`, the"]
    #[doc = " implementation may choose an appropriate default scheme. Fails if the"]
    #[doc = " string given is not a syntactically valid uri scheme."]
    fn set_scheme(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>, scheme: Option<Scheme>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    #[doc = " Get the HTTP Authority for the Request. A value of `none` may be used"]
    #[doc = " with Related Schemes which do not require an Authority. The HTTP and"]
    #[doc = " HTTPS schemes always require an authority."]
    fn authority(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    #[doc = " Set the HTTP Authority for the Request. A value of `none` may be used"]
    #[doc = " with Related Schemes which do not require an Authority. The HTTP and"]
    #[doc = " HTTPS schemes always require an authority. Fails if the string given is"]
    #[doc = " not a syntactically valid uri authority."]
    fn set_authority(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>, authority: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        todo!()
    }

    #[doc = " Get the headers associated with the Request."]
    #[doc = " "]
    #[doc = " The returned `headers` resource is immutable: `set`, `append`, and"]
    #[doc = " `delete` operations will fail with `header-error.immutable`."]
    #[doc = " "]
    #[doc = " This headers resource is a child: it must be dropped before the parent"]
    #[doc = " `outgoing-request` is dropped, or its ownership is transferred to"]
    #[doc = " another component by e.g. `outgoing-handler.handle`."]
    fn headers(
        &mut self, self_: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        todo!()
    }

    fn drop(
        &mut self, rep: wasmtime::component::Resource<OutgoingRequest>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::HostIncomingRequest for HermesState {
    #[doc = " Returns the method of the incoming request."]
    fn method(
        &mut self, self_: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Method> {
        todo!()
    }

    #[doc = " Returns the path with query parameters from the request, as a string."]
    fn path_with_query(
        &mut self, self_: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    #[doc = " Returns the protocol scheme from the request."]
    fn scheme(
        &mut self, self_: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<Scheme>> {
        todo!()
    }

    #[doc = " Returns the authority from the request, if it was present."]
    fn authority(
        &mut self, self_: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        todo!()
    }

    #[doc = " Get the `headers` associated with the request."]
    #[doc = " "]
    #[doc = " The returned `headers` resource is immutable: `set`, `append`, and"]
    #[doc = " `delete` operations will fail with `header-error.immutable`."]
    #[doc = " "]
    #[doc = " The `headers` returned are a child resource: it must be dropped before"]
    #[doc = " the parent `incoming-request` is dropped. Dropping this"]
    #[doc = " `incoming-request` before all children are dropped will trap."]
    fn headers(
        &mut self, self_: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Headers>> {
        todo!()
    }

    #[doc = " Gives the `incoming-body` associated with this request. Will only"]
    #[doc = " return success at most once, and subsequent calls will return error."]
    fn consume(
        &mut self, self_: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<IncomingBody>, ()>> {
        todo!()
    }

    fn drop(
        &mut self, rep: wasmtime::component::Resource<IncomingRequest>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl http::types::Host for HermesState {
    #[doc = " Attempts to extract a http-related `error` from the wasi:io `error`"]
    #[doc = " provided."]
    #[doc = " "]
    #[doc = " Stream operations which return"]
    #[doc = " `wasi:io/stream/stream-error::last-operation-failed` have a payload of"]
    #[doc = " type `wasi:io/error/error` with more information about the operation"]
    #[doc = " that failed. This payload can be passed through to this function to see"]
    #[doc = " if there\\'s http-related information about the error to return."]
    #[doc = " "]
    #[doc = " Note that this function is fallible because not all io-errors are"]
    #[doc = " http-related errors."]
    fn http_error_code(
        &mut self, err: wasmtime::component::Resource<IoError>,
    ) -> wasmtime::Result<Option<ErrorCode>> {
        todo!()
    }
}

impl http::outgoing_handler::Host for HermesState {
    #[doc = " This function is invoked with an outgoing HTTP Request, and it returns"]
    #[doc = " a resource `future-incoming-response` which represents an HTTP Response"]
    #[doc = " which may arrive in the future."]
    #[doc = " "]
    #[doc = " The `options` argument accepts optional parameters for the HTTP"]
    #[doc = " protocol\\'s transport layer."]
    #[doc = " "]
    #[doc = " This function may return an error if the `outgoing-request` is invalid"]
    #[doc = " or not allowed to be made. Otherwise, protocol errors are reported"]
    #[doc = " through the `future-incoming-response`."]
    fn handle(
        &mut self, request: wasmtime::component::Resource<OutgoingRequest>,
        options: Option<wasmtime::component::Resource<RequestOptions>>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<FutureIncomingResponse>, ErrorCode>>
    {
        todo!()
    }
}
