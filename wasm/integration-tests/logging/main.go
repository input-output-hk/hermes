package main

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

//go:generate wit-bindgen tiny-go ../wit --out-dir=hermes
func main() {
	// hermes.HermesLoggingApiLog(hermes.HermesLoggingApiLevelInfo(), hermes.None[string](), hermes.None[string](), hermes.None[uint32](), hermes.None[uint32](), hermes.None[string](), "Hello, world!", hermes.None[string]())
}
