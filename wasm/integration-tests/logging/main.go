package main

// FIXME: Rename this file
import (
	hermes "test/hermes"
)

func init() {
	a := WasiHttp0_2_0_IncomingHandler{}
	hermes.SetExportsWasiHttp0_2_0_IncomingHandler(a)
	b := HermesCardanoEventOnBlock{}
	hermes.SetExportsHermesCardanoEventOnBlock(b)
	c := HermesCardanoEventOnTxn{}
	hermes.SetExportsHermesCardanoEventOnTxn(c)
	d := HermesCardanoEventOnRollback{}
	hermes.SetExportsHermesCardanoEventOnRollback(d)
	e := HermesCronEvent{}
	hermes.SetExportsHermesCronEvent(e)
	f := HermesInitEvent{}
	hermes.SetExportsHermesInitEvent(f)
	g := HermesKvStoreEvent{}
	hermes.SetExportsHermesKvStoreEvent(g)
	h := HermesIntegrationTestEvent{}
	hermes.SetExportsHermesIntegrationTestEvent(h)
}

type WasiHttp0_2_0_IncomingHandler struct {
}

func (h WasiHttp0_2_0_IncomingHandler) Handle(request hermes.WasiHttp0_2_0_TypesIncomingRequest, response_out hermes.WasiHttp0_2_0_TypesResponseOutparam) {
}

type HermesCardanoEventOnBlock struct{}

func (h HermesCardanoEventOnBlock) OnCardanoBlock(blockchain hermes.ExportsHermesCardanoEventOnBlockCardanoBlockchainId, block hermes.ExportsHermesCardanoEventOnBlockCardanoBlock, source hermes.ExportsHermesCardanoEventOnBlockBlockSrc) {
}

type HermesCardanoEventOnTxn struct{}

func (h HermesCardanoEventOnTxn) OnCardanoTxn(blockchain hermes.ExportsHermesCardanoEventOnTxnCardanoBlockchainId, slot uint64, txn_index uint32, txn hermes.ExportsHermesCardanoEventOnTxnCardanoTxn) {
}

type HermesCardanoEventOnRollback struct{}

func (h HermesCardanoEventOnRollback) OnCardanoRollback(blockchain hermes.ExportsHermesCardanoEventOnRollbackCardanoBlockchainId, slot uint64) {
}

type HermesCronEvent struct{}

func (h HermesCronEvent) OnCron(event hermes.ExportsHermesCronEventCronTagged, last bool) bool {
	return false
}

type HermesInitEvent struct{}

func (h HermesInitEvent) Init() bool {
	return false
}

type HermesKvStoreEvent struct{}

func (h HermesKvStoreEvent) KvUpdate(key string, value hermes.ExportsHermesKvStoreEventKvValues) {
}

type HermesIntegrationTestEvent struct{}

func (h HermesIntegrationTestEvent) Test(test uint32, run bool) hermes.Option[hermes.ExportsHermesIntegrationTestEventTestResult] {
	return hermes.Option[hermes.ExportsHermesIntegrationTestEventTestResult]{}
}
func (h HermesIntegrationTestEvent) Bench(test uint32, run bool) hermes.Option[hermes.ExportsHermesIntegrationTestEventTestResult] {
	return hermes.Option[hermes.ExportsHermesIntegrationTestEventTestResult]{}
}

//go:generate wit-bindgen tiny-go ../../wasi/wit --out-dir=hermes
func main() {}
