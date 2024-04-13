use std::path::Path;

use futures::TryFutureExt;
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::ProtocolEncoding,
    common::APN,
    connection::Connection,
    msg::{data_obj_inp::DataObjInp, header::MsgType, stat::RodsObjStat},
};

use super::OprType;

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    fn make_stat_data_obj_in(path: &Path) -> DataObjInp {
        DataObjInp::new(path.to_str().unwrap().to_string(), OprType::No, 0, 0)
    }

    pub async fn stat(&mut self, path: &Path) -> Result<RodsObjStat, IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &Self::make_stat_data_obj_in(path),
                MsgType::RodsApiReq,
                APN::ObjStat as i32,
            )
            .await?;

        let (_, stat) = self
            .resources
            .get_header_and_msg::<T, RodsObjStat>()
            .await?;

        Ok(stat)
    }
}
