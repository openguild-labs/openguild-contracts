use ink::prelude::string::ToString;
use openbrush::{
    contracts::{
        ownable::{self, only_owner},
        psp34::{
            self,
            extensions::metadata::{self, PSP34MetadataImpl},
            Id, PSP34Error, PSP34Impl,
        },
        reentrancy_guard,
        reentrancy_guard::non_reentrant,
    },
    modifiers,
    traits::{AccountId, Balance, Storage, String},
};

use crate::types::SpaceNftError;

use super::types::SpaceNftData;

pub trait SpaceMarketInternal: Storage<SpaceNftData> + psp34::Internal {
    /// Check if the caller is owner of the token
    fn check_owner(&self, id: Id) -> Result<(), PSP34Error> {
        let owner = self._check_token_exists(&id.clone())?;
        let caller = Self::env().caller();
        if owner != caller {
            return Err(PSP34Error::Custom(SpaceNftError::NotTokenOwner.as_str()));
        }
        Ok(())
    }

    /// Check if the transferred mint value is as expected
    fn check_value(&self, transferred_value: u128) -> Result<(), PSP34Error> {
        if transferred_value != self.data::<SpaceNftData>().price_per_mint {
            return Err(PSP34Error::Custom(
                SpaceNftError::BadMintValue.as_str()
                    + "Required:"
                    + &self.data::<SpaceNftData>().price_per_mint.to_string()
                    + ", Supplied:"
                    + &transferred_value.to_string(),
            ));
        }

        if self.data::<SpaceNftData>().last_token_id >= self.data::<SpaceNftData>().max_supply {
            return Err(PSP34Error::Custom(SpaceNftError::CollectionIsFull.as_str()));
        }

        Ok(())
    }

    fn token_exists(&self, id: Id) -> Result<(), PSP34Error> {
        self._owner_of(&id).ok_or(PSP34Error::TokenNotExists)?;
        Ok(())
    }
}

#[openbrush::trait_definition]
pub trait SpaceMarketImpl:
    Storage<SpaceNftData>
    + Storage<reentrancy_guard::Data>
    + Storage<ownable::Data>
    + Storage<metadata::Data>
    + Storage<psp34::Data>
    + PSP34Impl
    + PSP34MetadataImpl
    + psp34::extensions::metadata::Internal
    + SpaceMarketInternal
{
    // caller mint nft to itself account
    #[ink(message, payable)]
    #[modifiers(non_reentrant)]
    fn mint(&mut self, fid: String) -> Result<Id, PSP34Error> {
        self.check_value(Self::env().transferred_value())?;

        let caller = Self::env().caller();
        let id = Id::U64(self.data::<SpaceNftData>().last_token_id + 1); // first mint id is 1
        self._mint_to(caller, id.clone())?;
        self.data::<SpaceNftData>().fid_list.insert(&id, &fid);
        self.data::<SpaceNftData>().last_token_id += 1;
        Ok(id)
    }

    // caller mint nft to another account
    #[ink(message, payable)]
    #[modifiers(non_reentrant)]
    fn mint_to(&mut self, to: AccountId, fid: String) -> Result<Id, PSP34Error> {
        self.check_value(Self::env().transferred_value())?;

        let id = Id::U64(self.data::<SpaceNftData>().last_token_id + 1); // first mint id is 1
        self._mint_to(to, id.clone())?;
        self.data::<SpaceNftData>().fid_list.insert(&id, &fid);
        self.data::<SpaceNftData>().last_token_id += 1;
        Ok(id)
    }

    // Set new value for the baseUri
    #[ink(message)]
    #[modifiers(only_owner)]
    fn set_base_uri(&mut self, uri: String) -> Result<(), PSP34Error> {
        let id = PSP34Impl::collection_id(self);
        metadata::Internal::_set_attribute(self, id, String::from("baseUri"), uri);
        Ok(())
    }

    // Set max supply of tokens
    #[ink(message)]
    #[modifiers(only_owner)]
    fn set_max_supply(&mut self, value: u64) -> Result<(), PSP34Error> {
        self.data::<SpaceNftData>().max_supply = value;
        Ok(())
    }

    // Get URI from token ID
    #[ink(message)]
    fn token_uri(&self, id: u64) -> Result<String, PSP34Error> {
        let id = Id::U64(id);
        self._check_token_exists(&id)?;
        let base_uri = PSP34MetadataImpl::get_attribute(
            self,
            PSP34Impl::collection_id(self),
            String::from("baseUri"),
        );
        let fid = self
            .data::<SpaceNftData>()
            .fid_list
            .get(&id)
            .ok_or(PSP34Error::TokenNotExists)?;

        let token_uri = base_uri.unwrap() + &fid;
        Ok(token_uri)
    }

    // Get token price
    #[ink(message)]
    fn price(&self, id: u64) -> Result<Balance, PSP34Error> {
        let id = Id::U64(id);
        let price = self
            .data::<SpaceNftData>()
            .sale_list
            .get(&id)
            .ok_or(PSP34Error::Custom(SpaceNftError::NotForSale.as_str()));
        price
    }

    // Get price per mint
    #[ink(message)]
    fn price_per_mint(&self) -> Balance {
        self.data::<SpaceNftData>().price_per_mint
    }

    // Get max supply of tokens
    #[ink(message)]
    fn max_supply(&self) -> u64 {
        self.data::<SpaceNftData>().max_supply
    }

    // Get Contract Balance
    #[ink(message)]
    fn balance(&mut self) -> Balance {
        let balance = Self::env().balance();
        let current_balance = balance
            .checked_sub(Self::env().minimum_balance())
            .unwrap_or_default();
        current_balance
    }

    /// Lists NFT for Sale
    #[ink(message)]
    fn list(&mut self, id: u64, price: Balance) -> Result<(), PSP34Error> {
        let id = Id::U64(id);
        self.check_owner(id.clone())?;
        self.data::<SpaceNftData>()
            .sale_list
            .insert(&id, &(price * 1_000_000_000_000));
        Ok(())
    }

    /// Delist NFT from Sale
    #[ink(message)]
    fn delist(&mut self, id: u64) -> Result<(), PSP34Error> {
        let id = Id::U64(id);
        self.check_owner(id.clone())?;
        if self.data::<SpaceNftData>().sale_list.get(&id).is_none() {
            return Err(PSP34Error::Custom(SpaceNftError::NotForSale.as_str()));
        }
        self.data::<SpaceNftData>().sale_list.remove(&id);
        Ok(())
    }

    /// Purchase NFT that is listed for Sale
    #[ink(message, payable)]
    fn purchase(&mut self, id: u64) -> Result<(), PSP34Error> {
        let id = Id::U64(id);
        let owner = self._check_token_exists(&id.clone())?;
        let caller = Self::env().caller();
        if owner == caller {
            return Err(PSP34Error::Custom(SpaceNftError::OwnToken.as_str()));
        };

        let price = self
            .data::<SpaceNftData>()
            .sale_list
            .get(&id)
            .ok_or(PSP34Error::Custom(SpaceNftError::NotForSale.as_str()))?;
        let transferred = Self::env().transferred_value();

        if price != transferred {
            return Err(PSP34Error::Custom(
                SpaceNftError::PriceNotMatch.as_str()
                    + "Required:"
                    + &price.to_string()
                    + ", Supplied:"
                    + &transferred.to_string(),
            ));
        }

        // Transfer native tokes
        if Self::env().transfer(owner, price).is_err() {
            return Err(PSP34Error::Custom(
                SpaceNftError::TransferNativeTokenFailed.as_str(),
            ));
        }

        self.data::<SpaceNftData>().sale_list.remove(&id);

        // Transfer NFT Token
        self._before_token_transfer(Some(&owner), Some(&caller), &id)?;
        self._remove_operator_approvals(&owner, &caller, &Some(&id));
        self._remove_token_owner(&id);
        self._insert_token_owner(&id, &caller);
        self._after_token_transfer(Some(&owner), Some(&caller), &id)?;
        self._emit_transfer_event(Some(owner), Some(caller), id.clone());

        Ok(())
    }

    /// Withdraws funds to contract owner
    #[ink(message)]
    #[modifiers(only_owner)]
    fn withdraw(&mut self) -> Result<(), PSP34Error> {
        let balance = Self::env().balance();
        let current_balance = balance
            .checked_sub(Self::env().minimum_balance())
            .unwrap_or_default();
        let owner = self.data::<ownable::Data>().owner.get().unwrap().unwrap();
        Self::env()
            .transfer(owner, current_balance)
            .map_err(|_| PSP34Error::Custom(SpaceNftError::WithdrawalFailed.as_str()))?;
        Ok(())
    }
}
