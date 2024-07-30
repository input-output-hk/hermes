package main

import hermes "hermes-golang-app-test/gen"

type TestModule struct{}

func (t TestModule) Handle(request hermes.ExportsWasiHttp0_2_0_IncomingHandlerIncomingRequest, responseOut hermes.ExportsWasiHttp0_2_0_IncomingHandlerResponseOutparam) {
}

func (t TestModule) KvUpdate(key string, value hermes.ExportsHermesKvStoreEventKvValues) {
}

func (t TestModule) Test(test uint32, run bool) hermes.Option[hermes.ExportsHermesIntegrationTestEventTestResult] {
	if test == 0 {
		return hermes.Some(hermes.ExportsHermesIntegrationTestEventTestResult{
			Name:   "Golang Test",
			Status: true,
		})
	}

	return hermes.None[hermes.ExportsHermesIntegrationTestEventTestResult]()
}

func (t TestModule) Bench(test uint32, run bool) hermes.Option[hermes.ExportsHermesIntegrationTestEventTestResult] {
	return hermes.None[hermes.ExportsHermesIntegrationTestEventTestResult]()
}

func (t TestModule) Init() bool {
	return true
}

func (t TestModule) OnCardanoTxn(blockchain hermes.ExportsHermesCardanoEventOnTxnCardanoBlockchainId, slot uint64, txnIndex uint32, txn hermes.ExportsHermesCardanoEventOnTxnCardanoTxn) {
}

func (t TestModule) OnCardanoBlock(blockchain hermes.ExportsHermesCardanoEventOnBlockCardanoBlockchainId, block hermes.ExportsHermesCardanoEventOnBlockCardanoBlock, source hermes.ExportsHermesCardanoEventOnBlockBlockSrc) {
}

func (t TestModule) OnCardanoRollback(blockchain hermes.ExportsHermesCardanoEventOnRollbackCardanoBlockchainId, slot uint64) {
}

func (t TestModule) OnCron(event hermes.ExportsHermesCronEventCronTagged, last bool) bool {
	return true
}

func init() {
	testModule := &TestModule{}
	hermes.SetExportsHermesCronEvent(testModule)
	hermes.SetExportsHermesCardanoEventOnRollback(testModule)
	hermes.SetExportsHermesCardanoEventOnBlock(testModule)
	hermes.SetExportsHermesCardanoEventOnTxn(testModule)
	hermes.SetExportsHermesInitEvent(testModule)
	hermes.SetExportsHermesIntegrationTestEvent(testModule)
	hermes.SetExportsHermesKvStoreEvent(testModule)
	hermes.SetExportsWasiHttp0_2_0_IncomingHandler(testModule)
}

func main() {}
