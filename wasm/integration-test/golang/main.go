package main

import (
	cardano "hermes-golang-app-test/binding/hermes/cardano/api"
	cardano_event_on_block "hermes-golang-app-test/binding/hermes/cardano/event-on-block"
	cardano_event_on_rollback "hermes-golang-app-test/binding/hermes/cardano/event-on-rollback"
	cardano_event_on_txn "hermes-golang-app-test/binding/hermes/cardano/event-on-txn"
	cron "hermes-golang-app-test/binding/hermes/cron/event"
	http_gateway "hermes-golang-app-test/binding/hermes/http-gateway/event"
	http_request "hermes-golang-app-test/binding/hermes/http-request/event"
	init_event "hermes-golang-app-test/binding/hermes/init/event"
	int_test "hermes-golang-app-test/binding/hermes/integration-test/event"
	ipfs "hermes-golang-app-test/binding/hermes/ipfs/event"
	kv "hermes-golang-app-test/binding/hermes/kv-store/event"
	http_incoming_handler "hermes-golang-app-test/binding/wasi/http/incoming-handler"

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

func (t TestModule) OnCardanoTxn(blockchain cardano.CardanoBlockchainID, slot uint64, txnIndex uint32, txn cardano.CardanoTxn) {
}

func (t TestModule) OnCardanoRollback(blockchain cardano.CardanoBlockchainID, slot uint64) {
}
func (t TestModule) OnCardanoBlock(blockchain cardano.CardanoBlockchainID, block cardano.CardanoBlock, source cardano.BlockSrc) {
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

func (t TestModule) Handle(request http_incoming_handler.ExportIncomingRequest, responseOut http_incoming_handler.ExportResponseOutparam) {
}

func init() {
	module := TestModule{}

	int_test.Exports.Test = module.Test
	int_test.Exports.Bench = module.Bench
	cardano_event_on_block.Exports.OnCardanoBlock = module.OnCardanoBlock
	cardano_event_on_rollback.Exports.OnCardanoRollback = module.OnCardanoRollback
	cardano_event_on_txn.Exports.OnCardanoTxn = module.OnCardanoTxn
	cron.Exports.OnCron = module.OnCron
	init_event.Exports.Init = module.Init
	ipfs.Exports.OnTopic = module.OnTopic
	kv.Exports.KvUpdate = module.KvUpdate
	http_gateway.Exports.Reply = module.Reply
	http_request.Exports.OnHTTPResponse = module.OnHTTPResponse
	http_incoming_handler.Exports.Handle = module.Handle
}

func main() {}