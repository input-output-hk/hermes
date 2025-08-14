// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod stub;

struct RbacRegistrationComponent;

impl hermes::exports::hermes::cardano::event_on_block::Guest for RbacRegistrationComponent {
}

impl hermes::exports::hermes::init::event::Guest for TestComponent {

}

func get_rbac_reg(raw_block: vec<u8>, network: String) {
    let pallas_block = cardano_blockchain_types::pallas_traverse::MultiEraBlock::decode(&raw_block).unwrap();

    let previous_point = cardano_blockchain_types::Point::new(
            (pallas_block.slot().checked_sub(1).unwrap()).into(),
            pallas_block
                .header()
                .previous_hash()
                .expect("cannot get previous hash")
                .into(),
        );
    let block = cardano_blockchain_types::MultiEraBlock::new(
            cardano_blockchain_types::Network::Preprod,
            block.raw(),
            &previous_point,
            // In this case, fork can be any
            1.into(),
        ).unwrap();

        let rbac_reg = rbac_registration::cardano::cip509::Cip509::from_block(&block, &[]);
}
hermes::export!(RbacRegistrationComponent with_types_in hermes);
