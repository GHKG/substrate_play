#![cfg_attr(not(feature = "std"), no_std)]

//=========================
use sp_core::crypto::KeyTypeId;
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"xkey");
pub mod crypto {
	use super::KEY_TYPE;
	use sp_runtime::app_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, KEY_TYPE);
}
pub type AuthorityId = crypto::Public;
//==========================

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	
	use frame_support::{
		dispatch::DispatchResult, //DispatchResultWithPostInfo,
		pallet_prelude::*,
		storage::child::MultiRemovalResults,
		traits::{Currency,ExistenceRequirement},
		//transactional,  //default behaviour for all extrinsics
	};
	use frame_system::pallet_prelude::*;
	//=========================
	use frame_system::offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
	};
	//=========================
	use scale_info::TypeInfo;
	use sp_runtime::traits::{IdentifyAccount, Verify};

	use sp_std::vec::Vec;
	use sp_core::{U256};
	//use sp_io::offchain;


	//#[cfg(feature = "std")]
	//use serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	//type BoundedVecOf<T> = BoundedVec<<T as frame_system::Config>::AccountId, <T as Config>::MaxMinersOwned>;
	const STATISTICSWINDOW : usize = 10;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	//#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum MinerClass {
		Game,
		Virtual,
		API,
		LitePV,
		StandardPV,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	//#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum MinerStatus<T: Config> {
		Locked,
		Registered,
		Onboard,
		AirDrop(AccountOf<T>, BlockNumberOf<T>),
		Expire,
		Terminated,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	//#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Energy {
		Light,
		Wind,
		Water,
		Other,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	//#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum EpochRewardStatus{
		Completed,
		Incomplete,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct EpochInfo{
		epoch_num: u32,
		reward_status: EpochRewardStatus,
	}


	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct MinerStaticData {
		//pub start_hight:BlockNumberOf<T> ,
		pub start_hight: u32,
		pub location: Vec<u8>,
		pub energy: Energy,
		pub capabilities: u32,
		pub device_model: Vec<u8>,
		pub serial_num: Vec<u8>,

		pub sg_voltage: u32,
		pub sg_current: u32,
		pub sg_power_low: u32,
		pub sg_power_high: u32,

		pub sb_voltage: u32,
		pub sb_current: u32,
		pub sb_power_low: u32,
		pub sb_power_high: u32,

		pub sl_voltage: u32,
		pub sl_current: u32,
		pub sl_power_low: u32,
		pub sl_power_high: u32,

		pub total_charge_vol: u32,
		pub total_usage_vol: u32,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Miner<T: Config> {
		pub class: MinerClass,
		pub maker: Vec<u8>,
		pub address: AccountOf<T>,
		pub status: MinerStatus<T>,
		pub owner: AccountOf<T>,
		pub location: Vec<u8>,
		pub staking_fee: u128,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct MinerEpoches{
		pub epochs: [u8;STATISTICSWINDOW],
		pub last_epoch: u32,
		pub total_count: u32,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebugNoBound, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct MinerData<T: Config> {
		pub miner: AccountOf<T>,
		pub data: u32,
	}

	type MinerRewards<T> = MinerData<T>;

	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {

		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		/// A Signature can be verified with a specific `PublicKey`.
		/// The additional traits are boilerplate.
		type Signature1: Verify<Signer = Self::PublicKey1> + Encode + Decode + Member + TypeInfo;
		/// A PublicKey can be converted into an `AccountId`. This is required by the
		/// `Signature` type.
		/// The additional traits are boilerplate.
		type PublicKey1: IdentifyAccount<AccountId = Self::PublicKey1>+ Encode+ Decode+ Member+ TypeInfo;
		#[pallet::constant]
		type MaxMinersOwned: Get<u32>;
		#[pallet::constant]
		type EpochCycleBlocks: Get<u32>;
		#[pallet::constant]
		type EpochTotalRewards: Get<u32>;
		
		type Currency: Currency<Self::AccountId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;	
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	//#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_miners)]
	pub type Miners<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Miner<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_static_data)]
	pub type StaticData<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, MinerStaticData>;

	#[pallet::storage]
	#[pallet::getter(fn get_admin)]
	pub type Admin<T: Config> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn get_owned_miners)]
	//pub type OwnedMiners<T:Config> =  StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<T::AccountId, T::MaxMinersOwned>>;
	pub type OwnedMiners<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn get_ari_drop_miners)]
	pub type AirDropExpire<T: Config> =	StorageMap<_, Twox64Concat, T::BlockNumber, Vec<T::AccountId>>;

	//used for  recording the past 168 epoch data of each miner
	#[pallet::storage]
	#[pallet::getter(fn get_miners_epoc_data)]
	pub type HistoryEpochData<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, MinerEpoches>;

	//used for epoch count
	#[pallet::storage]
	pub type CurrentEpocCycleInfo<T: Config> = StorageValue<_, EpochInfo>;

	//used for  recording miner effective data  in current epoch
	#[pallet::storage]
	pub type CurrentEpochData<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u32>;

	//collection of current epoch miners who upload information
	#[pallet::storage]
	pub type CurrentEpochMiners<T: Config> = StorageValue<_, Vec<T::AccountId>>;

	//OCW account used for signing extrinics from offchain
	#[pallet::storage]
	pub type Ocw<T: Config> = StorageValue<_, T::AccountId>;

	//foundations account  who transfer tokens to miners as rewards
	#[pallet::storage]
	pub type Foundations<T: Config> = StorageValue<_, T::AccountId>;


	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
		NewMinerRegisted(T::AccountId),
		AdminIsSet(T::AccountId),
		MinerOnBoard(T::AccountId, Vec<u8>, u128),
	}

	#[pallet::error]
	pub enum Error<T> {
		NoneValue,
		StorageOverflow,
		AdminIsNotSet,
		CallerIsNotAdmin,
		MinerIsNotExist,
		OwnerNotMatch,
		ExceedMaxMainerOwned,
		AlreadyOnBoard,
		NoOwnedVectorFound,
		MinerNotFound,
		NoDropVectorFound,
		StatusIsNotRegistered,
		StatusIsNotAirDrop,
		CallerIsNotAirDropReceiver,
		SignatureVerifyError,
		OffchainSignedTxError,
		NoLocalAcctForSigning,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		///////////////////////////////////////////////////////////////////////////
		//  transfer some tokens to an account
		//////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn my_transfer(	origin: OriginFor<T>, destination: T::AccountId, amount: BalanceOf<T>,)-> DispatchResult{
			let owner = ensure_signed(origin)?;
			T::Currency::transfer(&owner, &destination, amount, ExistenceRequirement::KeepAlive)?;
			Ok(())
		}

		////////////////////////////////////////////////////////////////////////////
		//   set ocw account and transfer some tokens from supplyer, called by root account
		///////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn set_ocw_signer(
			origin: OriginFor<T>,
			ocw: T::AccountId,
			amount: BalanceOf<T>,
			supplyer: T::AccountId,
		)-> DispatchResult{
			ensure_root(origin)?;
			T::Currency::transfer(&supplyer, &ocw, amount, ExistenceRequirement::KeepAlive)?;
			<Ocw<T>>::put(ocw);
			Ok(())
		}

		/////////////////////////////////////////////////////////////////////////////
		//   set foundations account
		////////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn set_foundations(origin: OriginFor<T>, address: T::AccountId) -> DispatchResult {
			
			ensure_root(origin.clone())?;//only sudo user could do this

			<Foundations<T>>::put(&address);
			
			//Admin::<T>::put(address);
			Self::deposit_event(Event::AdminIsSet(address));

			Ok(())
		}
		
		////////////////////////////////////////////////////////////////////////////
		//    verify some data with accountId(public key) and signature
		////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn my_verify(
			_origin: OriginFor<T>,
			data: Vec<u8>,
			sig: T::Signature1,
			from: T::PublicKey1,
		) -> DispatchResult {
			//**************************************************************************
			//pub fn my_verify(origin: OriginFor<T>, data: Vec<u8>, sig: T::Signature, from: T::AccountId) -> DispatchResult{
			// let who = ensure_signed(origin)?;
			// // 提取公钥
			// let encode_data: Vec<u8> = from.encode();
			// // 公钥应该是32位的
			// assert!(32 == encode_data.len());
			// let raw_data = encode_data.try_into();
			// let raw_data = raw_data.unwrap();
			// let public = sp_core::sr25519::Public::from_raw(raw_data);
			// log::info!("public key is {:?}", public);
			// log::info!("--------public key is {:?}", from.clone().into_account());
			// log::info!("--------signature is {:?}", sig.clone());
			// log::info!("--------data is {:?}", data.clone());
			//***************************************************************************
			let ok = sig.verify(&data[..], &from.into_account());
			// `ok` is a bool. Use in an `if` or `ensure!`.
			ensure!(ok, <Error<T>>::SignatureVerifyError);
			Ok(())
		}

		//***************************************************************************
		// #[pallet::weight(1234)]
		// pub fn reveal_puzzle(
		// 	origin: OriginFor<T>,
		// 	answer_signed: Vec<u8>,
		// 	answer_hash: Vec<u8>,
		// ) -> DispatchResult {
		// 	let who = ensure_signed(origin)?;
		// 	// 提取公钥
		// 	let encode_data: Vec<u8> = who.encode();
		// 	// 公钥应该是32位的
		// 	assert!(32 == encode_data.len());
		// 	let raw_data = encode_data.try_into();
		// 	let raw_data = raw_data.unwrap();
		// 	let public = sp_core::sr25519::Public::from_raw(raw_data);
		// 	Self::check_signed_valid(
		// 		public,
		// 		// 这边要注意先要吧签名的16进制decode成&[u8]
		// 		&hex::decode(answer_signed.as_slice()).expect("Hex invalid")[..],
		// 		answer_hash.as_slice(),
		// 	);
		// 	Ok(())
		// }
		//***************************************************************************
		
		


		//////////////////////////////////////////////////////////////////////////////
		//register a miner by administrator
		///////////////////////////////////////////////////////////////////////////////

		//pub fn register_miner(origin: OriginFor<T>, class: MinerClass, maker: Vec<u8>, address: AccountOf<T>) -> DispatchResult {
		//#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		#[pallet::weight(10_000)]
		pub fn register_miner(
			origin: OriginFor<T>,
			class: MinerClass,
			maker: Vec<u8>,
			address: T::AccountId,
		) -> DispatchResult {

			let who = ensure_signed(origin)?;

			match <Admin<T>>::get() {
				Some(admin) => {
					ensure!(admin == who, <Error<T>>::CallerIsNotAdmin);
				},
				None => return Err(Error::<T>::AdminIsNotSet.into()),
			};

			assert!(!<Miners<T>>::contains_key(&address));

			let new_miner = Miner {
				class,
				maker,
				address: address.clone(),
				owner: address.clone(),
				location: Vec::<u8>::new(),
				staking_fee: 0u128,
				status: MinerStatus::Registered,
			};

			<Miners<T>>::insert(&address, new_miner);

			Self::deposit_event(Event::NewMinerRegisted(address));
			Ok(())
		}


		//////////////////////////////////////////////////////////////////////////////////
		//   set administrator account who has high authority
		/////////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn set_admin(origin: OriginFor<T>, address: T::AccountId) -> DispatchResult {
			
			ensure_root(origin.clone())?;//only sudo user could do this

			<Admin<T>>::put(&address);
			// another code style
			//Admin::<T>::put(address);
			Self::deposit_event(Event::AdminIsSet(address));

			Ok(())
		}

		////////////////////////////////////////////////////////////////////////////
		//   on board a miner by owner 
		////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn on_board(
			origin: OriginFor<T>,
			miner_address: T::AccountId,
			location: Vec<u8>,
			staking_fee: u128,
			static_data: MinerStaticData,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			match <Miners<T>>::get(&miner_address) {
				Some(mut miner) => {
					ensure!(miner.status == MinerStatus::Registered, Error::<T>::AlreadyOnBoard);
					miner.status = MinerStatus::Onboard;
					miner.owner = owner.clone();
					miner.location = location;
					miner.staking_fee = staking_fee;

					//write back updated information
					<Miners<T>>::insert(&miner_address, miner);

					let new_static_data = MinerStaticData {
						start_hight: <frame_system::Pallet<T>>::block_number().try_into().unwrap_or(0),
						..static_data
					};

					//write new_static_data
					<StaticData<T>>::insert(&miner_address, new_static_data);

					//********************************************************************
					// <OwnedMiners<T>>::mutate(owner, |vec| match vec {
					// 	None => Err(()),
					// 	Some(vector) => {
					// 		vector.push(miner_address);
					// 		Ok(())
					// 	},
					// }).map_err(|_| <Error<T>>::NoOwnedVectorFound)?;
					//********************************************************************

					//mutate owned vector or create a vector for the first time
					if <OwnedMiners<T>>::contains_key(&owner) {
						<OwnedMiners<T>>::mutate(&owner, |vec| match vec {
							None => Err(()),
							Some(v) => {
								assert!((v.len() as u32) < T::MaxMinersOwned::get());
								v.push(miner_address);
								Ok(().into())
							},
						}).map_err(|_| <Error<T>>::NoOwnedVectorFound)?;

					} else {
						let mut miners_vec: Vec<T::AccountId> = Vec::new();
						miners_vec.push(miner_address);
						<OwnedMiners<T>>::insert(owner, miners_vec);
					}
				},
				None => return Err(Error::<T>::MinerIsNotExist.into()),
			}

			Ok(())
		}

		////////////////////////////////////////////////////////////////////////////
		//    transfer a miner to another owner
		////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn transfer_miner(
			origin: OriginFor<T>,
			miner_address: T::AccountId,
			receiver: T::AccountId,
		) -> DispatchResult {
			let current_owner = ensure_signed(origin)?;

			match <Miners<T>>::get(&miner_address) {
				Some(mut miner) => {
					assert!(miner.status == MinerStatus::Onboard);
					ensure!(miner.owner == current_owner, Error::<T>::OwnerNotMatch);

					<OwnedMiners<T>>::try_mutate(&current_owner, |owned| match owned {
						None => Err(()),
						Some(v) => {
							if let Some(ind) = v.iter().position(|id| id == &miner_address) {
								v.swap_remove(ind);
								Ok(())
							} else {
								Err(())
							}
						},
					})
					.map_err(|_| <Error<T>>::NoOwnedVectorFound)?;

					//makesure that the receiver got the miner
					if <OwnedMiners<T>>::contains_key(&receiver) {
						<OwnedMiners<T>>::mutate(&receiver, |vec| match vec {
							None => Err(()),
							Some(v) => {
								assert!((v.len() as u32) < T::MaxMinersOwned::get());
								v.push(miner_address.clone());
								Ok(())
							},
						})
						.map_err(|_| <Error<T>>::NoOwnedVectorFound)?;
					} else {
						let mut miners_vec: Vec<T::AccountId> = Vec::new();
						miners_vec.push(miner_address.clone());
						<OwnedMiners<T>>::insert(&receiver, miners_vec);
					}

					//at last , change the owner of miner
					miner.owner = receiver;
					<Miners<T>>::insert(&miner_address, miner);
				},
				None => return Err(Error::<T>::MinerIsNotExist.into()),
			}

			Ok(())
		}

		///////////////////////////////////////////////////////////////////////////////
		//   terminate a miner by administrator
		//////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn terminate_miner(
			origin: OriginFor<T>,
			miner_address: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			match <Admin<T>>::get() {
				Some(admin) => {
					ensure!(admin == who, <Error<T>>::CallerIsNotAdmin);
				},
				// 提前返回
				None => return Err(Error::<T>::AdminIsNotSet.into()),
			};

			//***********************************************************************
			// match <Miners<T>>::get(&miner_address){
			// 	Some(mut miner) => {
			// 		miner.status = MinerStatus::Terminated;
			// 		<Miners<T>>::insert(&miner_address, miner);
			// 	},
			// 	None => return Err(Error::<T>::MinerIsNotExist.into()),
			// }
			//***********************************************************************

			<Miners<T>>::try_mutate(&miner_address, |miner| match miner {
				None => Err(()),
				Some(registered) => {
					registered.status = MinerStatus::Terminated;
					Ok(())
				},
			})
			.map_err(|_| <Error<T>>::MinerIsNotExist)?;

			Ok(())
		}

		////////////////////////////////////////////////////////////////////////////
		//     air drop a miner to an owner  by administrator
		////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn air_drop(
			origin: OriginFor<T>,
			miner_address: T::AccountId,
			receiver: T::AccountId,
			life_time: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			match <Admin<T>>::get() {
				Some(admin) => {
					ensure!(admin == who, <Error<T>>::CallerIsNotAdmin);
				},
				None => return Err(Error::<T>::AdminIsNotSet.into()),
			};

			match <Miners<T>>::get(&miner_address) {
				None => return Err(Error::<T>::MinerIsNotExist.into()),
				Some(mut registered) => {
					ensure!(
						registered.status == MinerStatus::Registered,
						Error::<T>::StatusIsNotRegistered
					);
					//assert!(registered.status == MinerStatus::Registered);

					//insert miner addr to exprie
					let expire_block_num: T::BlockNumber =
						<frame_system::Pallet<T>>::block_number() + life_time;
					if <AirDropExpire<T>>::contains_key(expire_block_num) {
						<AirDropExpire<T>>::mutate(expire_block_num, |vec| match vec {
							None => Err(()),
							Some(v) => {
								//assert!((v.len() as u32) < T::MaxMinersOwned::get());
								v.push(miner_address.clone());
								Ok(())
							},
						})
						.map_err(|_| <Error<T>>::NoDropVectorFound)?;
					} else {
						let mut miners_vec: Vec<T::AccountId> = Vec::new();
						miners_vec.push(miner_address.clone());
						<AirDropExpire<T>>::insert(expire_block_num, miners_vec);
					}

					//write back updated information
					registered.status = MinerStatus::AirDrop(receiver.clone(), expire_block_num);
					<Miners<T>>::insert(miner_address, registered);
				},
			}
			Ok(())
		}

		////////////////////////////////////////////////////////////////////////////
		//     an owner  apply for a miner
		////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn claim(origin: OriginFor<T>, miner_address: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// if <Miners<T>>::contains_key(&miner_address){
			// 	return Err(Error::<T>::MinerIsNotExist.into());
			// }

			match <Miners<T>>::get(&miner_address) {
				None => return Err(Error::<T>::MinerIsNotExist.into()),
				Some(mut registered) => {
					match registered.status {
						MinerStatus::AirDrop(account, blocknum) => {
							ensure!(account == who, <Error<T>>::CallerIsNotAirDropReceiver);
							registered.status = MinerStatus::Onboard;
							registered.owner = who.clone();
							// registered.location = ;
							// registered.staking_fee = staking_fee;
							<Miners<T>>::insert(&miner_address, registered);

							let new_static_data = MinerStaticData {
								start_hight: <frame_system::Pallet<T>>::block_number().try_into().unwrap_or(0),

								location: Vec::<u8>::new(),
								energy: Energy::Other,
								capabilities: 0,
								device_model: Vec::<u8>::new(),
								serial_num: Vec::<u8>::new(),

								sg_voltage: 0,
								sg_current: 0,
								sg_power_low: 0,
								sg_power_high: 0,

								sb_voltage: 0,
								sb_current: 0,
								sb_power_low: 0,
								sb_power_high: 0,

								sl_voltage: 0,
								sl_current: 0,
								sl_power_low: 0,
								sl_power_high: 0,

								total_charge_vol: 0,
								total_usage_vol: 0,
							};

							//write new_static_data
							<StaticData<T>>::insert(&miner_address, new_static_data);

							<AirDropExpire<T>>::try_mutate(blocknum, |drop_vec| match drop_vec {
								None => Err(()),
								Some(v) => {
									if let Some(ind) =
										v.iter().position(|addr| addr == &miner_address)
									{
										v.swap_remove(ind);
									};
									Ok(())
								},
							})
							.map_err(|_| <Error<T>>::NoDropVectorFound)?;

							// <AirDropExpire<T>>::try_mutate(blocknum, |drop_vec| match drop_vec {
							// 	None => Err(()),
							// 	Some(v) => {
							// 		if let Some(ind) = v.iter().position(|addr| addr == &miner_address) {
							// 			v.swap_remove(ind);
							// 			Ok(())
							// 		}else{
							// 			Err(())
							// 		}
							// 	}
							// }).map_err(|_| <Error<T>>::NoDropVectorFound)?;

							Ok(())
						},
						_ => return Err(Error::<T>::StatusIsNotAirDrop.into()),
					}
				},
			}
		}

		
		////////////////////////////////////////////////////////////////////////
		//   reward miners from offchain extrinsic by ocw account
		//////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn reward_miners(
			origin: OriginFor<T>,
			epoch_rewards: Vec<MinerData<T>>,
			reward_epoch_number: u32,
		  ) -> DispatchResult {

			let who = ensure_signed(origin)?;

			match <Ocw<T>>::get(){
				None => return Err(Error::<T>::StatusIsNotAirDrop.into()),
				Some(aid) => {if aid != who {return Err(Error::<T>::StatusIsNotAirDrop.into())}},
			}

			//let foundations = <Foundations<T>>::get().is_some();
			//if let None = foundations {return Err(Error::<T>::StatusIsNotAirDrop.into())}
			if <Foundations<T>>::get().is_none() {
				return Err(Error::<T>::StatusIsNotAirDrop.into());
			}


			let current_epoch_cycle_info = <CurrentEpocCycleInfo<T>>::get().unwrap();
			if current_epoch_cycle_info.epoch_num != reward_epoch_number
				|| current_epoch_cycle_info.reward_status == EpochRewardStatus::Completed{
					return Err(Error::<T>::StatusIsNotAirDrop.into());
				}

			// if let Some(False) = <CurrentEpocCycleInfo<T>>::get().map(|ceci| {
			// 	if ceci.epoch_num != reward_epoch_number || ceci.reward_status == EpochRewardStatus::Completed {
			// 		return False;
			// 	}
			// }){
			// 	return Err(Error::<T>::StatusIsNotAirDrop.into());
			// }

			let foundations = <Foundations<T>>::get().unwrap();

			for mr in epoch_rewards{
				//log::info!(target: "on chain", "epoch reward miner  {:?} --- reward {:?}", mr.miner, mr.data);
				T::Currency::transfer(&foundations, &mr.miner, BalanceOf::<T>::from(mr.data), ExistenceRequirement::KeepAlive)?;

			}

			Ok(())
		}

		//////////////////////////////////////////////////////////////////////////////////////////////
		//    miner report information in epoch cycle
		////////////////////////////////////////////////////////////////////////////////////////////
		#[pallet::weight(10_000)]
		pub fn miner_report(origin: OriginFor<T>, effectivedata:u8) -> DispatchResult{

			let miner = ensure_signed(origin)?;

			match <CurrentEpochData<T>>::try_get(&miner) {
				Err(_) => <CurrentEpochData<T>>::insert(miner, effectivedata as u32),
				Ok(v) => {
					<CurrentEpochData<T>>::insert(miner, v + (effectivedata as u32))
				}
			}

			Ok(())

		}

	}

	

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

		///////////////////////////////////////////////////////////////////
		//    handle air drop expire logic
		//////////////////////////////////////////////////////////////////
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {

			if <AirDropExpire<T>>::contains_key(n) {
				match <AirDropExpire<T>>::get(n) {
					None => return 0,
					Some(v) => {
						for miner_address in v {
							match <Miners<T>>::get(&miner_address) {
								None => return 0,
								Some(mut miner) => {
									if miner.status != MinerStatus::Onboard {
										miner.status = MinerStatus::Expire;
									}
									//write change back
									<Miners<T>>::insert(&miner_address, miner);
								},
							}
						}
						<AirDropExpire<T>>::remove(n);
					},
				}
			}

			return 0;
		}

		////////////////////////////////////////////////////////////////////////////
		//   trigger epoch cycle action: 
		//////////////////////////////////////////////////////////////////////////
		fn on_finalize(n: BlockNumberFor<T>){

			let block_num:u32 = n.try_into().unwrap_or(0);

			//should not trigger cycle action
			if ((block_num % T::EpochCycleBlocks::get()) != 0) || block_num == 0 {
				return;
			}
			
			/////////////////start cycle action//////////////////////

			let epoch_num  = block_num / T::EpochCycleBlocks::get();

			//record current epoch number and status
			let epoch_info = EpochInfo{epoch_num:epoch_num, reward_status:EpochRewardStatus::Incomplete};
			<CurrentEpocCycleInfo<T>>::put(epoch_info);

			//create an empty vec for current epoch action
			<CurrentEpochMiners<T>>::put(Vec::<T::AccountId>::new());
		
			let window_size:u32 = STATISTICSWINDOW as u32;//For convenience

			//update history data with current epoch data
			for (miner, value) in <CurrentEpochData<T>>::iter(){

				match <HistoryEpochData<T>>::try_get(&miner){
					// handle new miner information 
					Err(_) => {
						let mut new_record = MinerEpoches{
							epochs: [0; STATISTICSWINDOW],
							last_epoch: epoch_num,
							total_count: value,
						};
						new_record.epochs[(epoch_num % window_size) as usize] = if value >= 12{ 12 } else {value as u8};
						<HistoryEpochData<T>>::insert(&miner, new_record);
					},
					// handle miner information already recorded before
					Ok(s) =>{

						//in order to make code  well-organized and clear, we use "get-modify-write back" mode. 

						let mut update_record = s.clone();
						let skip_distance = epoch_num - update_record.last_epoch ;
						let mut count = if skip_distance >= window_size {window_size} else {skip_distance - 1};
						//set range 0
						while count > 0 {
							update_record.epochs[((update_record.last_epoch + count) % window_size) as usize] = 0;
							count = count - 1;
						}

						//at last ,write the value in correct position
						update_record.epochs[(epoch_num % window_size) as usize] = if value >= 12{ 12 } else {value as u8};

						//update last epoch number
						update_record.last_epoch = epoch_num;

						//maybe overflow so should not use iter().sum()
						        //update_record.total_count = update_record.epochs.iter().sum::<u32>();

						let mut total_count:u32 = 0;
						for i in update_record.epochs.iter(){
							total_count += *i as u32;
						}
						update_record.total_count = total_count;

						//write back to update history 168 data , (for test we use 10 except 168)
						<HistoryEpochData<T>>::insert(&miner, update_record);

					}

					//*************************************************************************
					// Ok(mut s) =>{
					// 	//calculate the span index of items which should be set 0
					// 	let skip_distance = epoch_num - s.last_epoch ;
					// 	let mut count = if skip_distance >= 168 {168} else {skip_distance - 1};
					// 	//set range 0
					// 	while count > 0 {
					// 		s.epochs[((s.last_epoch + count) % 168) as usize] = 0;
					// 		count = count - 1;
					// 	}

					// 	//at last ,write the value in correct position
					// 	s.epochs[(epoch_num % 168) as usize] = if value >= 12{ 12 } else {value as u8};

					// 	s.last_epoch = epoch_num;
					// 	//s.total_count = s.epochs.iter().sum::<u32>();//maybe overflow as max is 12*168=2016
					// 	let mut total_count = 0;
					// 	for i in s.epochs.iter(){
					// 		//total_count = total_count + (*i as u32);
					// 		total_count += *i as u32;
					// 	}
					// 	s.total_count = total_count;
						
						
					// }
					//*****************************************************************************
				}

				//   record current epoch cycle miners for offchain use
				<CurrentEpochMiners<T>>::mutate(|vect| {
					if let Some(v) = vect {
						v.push(miner);
					}});
			}

			//clear current epoch data for the next cycle
			//We could also use double buffer here to get rid off the time or weight limition. 
			let mut mrr: MultiRemovalResults = <CurrentEpochData<T>>::clear(10000, None);
			loop {
				match mrr.maybe_cursor{
					None => break,
					Some(v) => {
						mrr = <CurrentEpochData<T>>::clear(10000, Some(&v[..]));
					}
				}
			}


		}

		/////////////////////////////////////////////////////////////////////
		//  offchain work:
		//     1.reward Qualification determination
		//     2.calculate reward amount
		//     3.send extrinsic to chain vith signed transaction  
		/////////////////////////////////////////////////////////////////////
		fn offchain_worker(n: BlockNumberFor<T>) {
			
			//make sure start action at (block_num % T::EpochCycleBlocks::get()) == 1
			let block_num: u32 = n.try_into().unwrap_or(0);
			if (block_num <= T::EpochCycleBlocks::get()) || ((block_num % T::EpochCycleBlocks::get()) != 1) {
				return;
			}

			//get last epoch miners
			let epoch_miners = <CurrentEpochMiners<T>>::get();
			match &epoch_miners{
				None => return,
				Some(v) => if v.len()==0 {return;},
			}

			let epoch_num = <CurrentEpocCycleInfo<T>>::get().unwrap().epoch_num;

			///////////filter miners who could get reward//////////////////

			//collect latest 60 block hashs, for test use 5
			let mut epoch_hashs:Vec<u8> = Vec::new();
			for i in (epoch_num-1)*5+1..=epoch_num*5 {
				epoch_hashs.extend_from_slice(&<frame_system::Pallet<T>>::block_hash(BlockNumberOf::<T>::from(i)).encode());
			}

			//cut the umbilical cord attached to the on-chain storage
			let epoch_miners = epoch_miners.unwrap().clone();

			//generate random seed
			let random_seed = sp_io::offchain::random_seed();
			let random_number:U256 = U256::from_little_endian(&random_seed);

			let mut hash_data_buf:[u8;32*5 + 32] = [0; 32*5 + 32];//60 Facing the future, test use 5
			for i in 0..epoch_hashs.len(){
				hash_data_buf[i+32] = epoch_hashs[i];
			}


			let mut epoch_reward_miners: Vec<MinerData<T>> = Vec::new();//MinerData record miner address and its amount data 
			let mut epoch_miner_total_count:u32 = 0;
			for miner in epoch_miners{

				let miner_total_count = <HistoryEpochData<T>>::get(&miner).unwrap().total_count;
				let miner_number:U256 = U256::from_little_endian(&miner.encode());
				//let part_data = (random_number ^ miner_number) * U512::from(miner_total_count);
				let part_data = random_number ^ miner_number ^ U256::from(miner_total_count);
				let part_data:[u8;32] = part_data.into();

				for i in 0..32 {
					hash_data_buf[i] = part_data[i];
				}

				let final_hash =  sp_io::hashing::sha2_256(&hash_data_buf);

				let big_num = U256::from(final_hash);

				//mod 60 to valid Qualification, 2 to test
				if (big_num  % 2) == U256::from(0 as u32) {
					let miner_data = MinerData{
						miner: miner,
						data: miner_total_count,
					};
					epoch_reward_miners.push(miner_data);
					epoch_miner_total_count += miner_total_count;
				}
			}

			// no miner could get reward
			if epoch_reward_miners.len() == 0{
				return;
			}

			log::info!(target: "ocw", " {:?} miners share total {:?} tokens", epoch_reward_miners.len() , T::EpochTotalRewards::get());


			//calculate each miner reward by weight
			let epoch_total_rewards = T::EpochTotalRewards::get();
			for mr in epoch_reward_miners.iter_mut(){
				//log::info!(target: "ocw", "miner  {:?} got [ {:?}/{:?} ] total tokens", mr.miner, mr.data, epoch_miner_total_count);
				mr.data = mr.data * epoch_total_rewards / epoch_miner_total_count;
				//log::info!(target: "ocw", "about {:?} tokens", mr.data);
			}

			//send reward info to blockchain in signed  extrinsic
			let result = Self::offchain_signed_tx_reward_miners(epoch_reward_miners, epoch_num);

			if let Err(e) = result {
				log::error!(target:"ocw", "offchain_worker error: {:?}", e);
			}

		}
	}


	impl<T: Config> Pallet<T> {
		fn offchain_signed_tx_reward_miners(epoch_rewards: Vec<MinerData<T>>, epoch_num:u32) -> Result<(), Error<T>> {


			let signer = Signer::<T, T::AuthorityId>::any_account();
			log::info!(target:"ocw", "+++++++++++++++++++, can sign: {:?}", signer.can_sign());
							  
			let result =
			  signer.send_signed_transaction(|_acct| Call::reward_miners { 
				epoch_rewards:epoch_rewards.clone() ,reward_epoch_number: epoch_num});
	  
			if let Some((_acc, res)) = result {
			  if res.is_err() {
				return Err(<Error<T>>::OffchainSignedTxError)
			  }
			  Ok(())
			} else {
			  Err(<Error<T>>::NoLocalAcctForSigning)
			}
		  }
	}
}
