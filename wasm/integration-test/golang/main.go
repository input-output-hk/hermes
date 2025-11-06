package main

import (
	cardano_event_on_block "hermes-golang-app-test/binding/hermes/cardano/event-on-block"
	cardano_event_on_immutable_roll_forward "hermes-golang-app-test/binding/hermes/cardano/event-on-immutable-roll-forward"
	cron "hermes-golang-app-test/binding/hermes/cron/event"
	doc_sync "hermes-golang-app-test/binding/hermes/doc-sync/event"
	http_gateway "hermes-golang-app-test/binding/hermes/http-gateway/event"
	auth "hermes-golang-app-test/binding/hermes/http-gateway/event-auth"
	http_request "hermes-golang-app-test/binding/hermes/http-request/event"
	init_event "hermes-golang-app-test/binding/hermes/init/event"
	int_test "hermes-golang-app-test/binding/hermes/integration-test/event"
	ipfs "hermes-golang-app-test/binding/hermes/ipfs/event"
	kv "hermes-golang-app-test/binding/hermes/kv-store/event"

	"go.bytecodealliance.org/cm"
)

type TestModule struct{}

func (t TestModule) Test(test uint32, run bool) cm.Option[int_test.TestResult] {
	return cm.Some(int_test.TestResult{
		Name:   "Golang Test",
		Status: true,
	})
}

func (t TestModule) Bench(test uint32, run bool) cm.Option[int_test.TestResult] {
	return cm.None[int_test.TestResult]()
}

func (t TestModule) OnCardanoBlock(subscription_id cm.Rep, block cm.Rep) {
}

func (t TestModule) OnCardanoImmutableRollForward(subscription_id cm.Rep, block cm.Rep) {
}

func (t TestModule) OnCron(event cron.CronTagged, last bool) bool {
	return true
}

func (t TestModule) Init() bool {
	return true
}

func (t TestModule) OnTopic(message ipfs.PubsubMessage) bool {
	return true
}

func (t TestModule) KvUpdate(key string, value kv.KvValues) {}

func (t TestModule) Reply(body http_gateway.Bstr, headers http_gateway.Headers, path string, method string) cm.Option[http_gateway.HTTPGatewayResponse] {
	return cm.None[http_gateway.HTTPGatewayResponse]()
}

func (t TestModule) OnHTTPResponse(requestID cm.Option[uint64], response cm.List[uint8]) {}

func (t TestModule) ValidateAuth(request auth.AuthRequest) cm.Option[auth.HTTPResponse] {
	return cm.None[auth.HTTPResponse]()
}

func (t TestModule) OnNewDoc(channel doc_sync.ChannelName, doc doc_sync.DocData) {}


func init() {
	module := TestModule{}

	int_test.Exports.Test = module.Test
	int_test.Exports.Bench = module.Bench
	cardano_event_on_block.Exports.OnCardanoBlock = module.OnCardanoBlock
	cardano_event_on_immutable_roll_forward.Exports.OnCardanoImmutableRollForward = module.OnCardanoImmutableRollForward
	cron.Exports.OnCron = module.OnCron
	init_event.Exports.Init = module.Init
	ipfs.Exports.OnTopic = module.OnTopic
	kv.Exports.KvUpdate = module.KvUpdate
	http_gateway.Exports.Reply = module.Reply
	http_request.Exports.OnHTTPResponse = module.OnHTTPResponse
	auth.Exports.ValidateAuth = module.ValidateAuth
	doc_sync.Exports.OnNewDoc = module.OnNewDoc
}

func main() {}
