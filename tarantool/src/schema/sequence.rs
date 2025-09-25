use crate::error::Error;
use crate::schema;
use crate::sequence::Sequence;
use crate::space::{Space, SystemSpace};
use tlua::{
    LuaError::{self, ExecutionError},
    LuaFunction, LuaTable,
};

/// Drop existing sequence.
///
/// - `seq_id` - ID of existing space.
pub fn drop_sequence(seq_id: u32) -> Result<(), Error> {
    schema::revoke_object_privileges("sequence", seq_id)?;

    let sys_sequence_data: Space = SystemSpace::SequenceData.into();
    sys_sequence_data.delete(&(seq_id,))?;

    let sys_sequence: Space = SystemSpace::Sequence.into();
    sys_sequence.delete(&(seq_id,))?;

    Ok(())
}

/// Sequence create options.
///
/// For details see [schema.create_sequence](https://www.tarantool.io/en/doc/latest/reference/reference_lua/box_schema_sequence/create/)
#[derive(Debug, Clone, Default, tlua::Push)]
pub struct SequenceCreateOptions {
    pub cache: Option<i64>,
    pub cycle: Option<bool>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub start: Option<i64>,
    pub step: Option<i64>,
}

/// Create new sequence.
///
/// - `seq_name`   - name of sequence to create.
/// - `opts`       - see [`SequenceCreateOptions`] struct.
///
/// For details see [schema.create_sequence](https://www.tarantool.io/en/doc/latest/reference/reference_lua/box_schema_sequence/create/)
pub fn create_sequence(seq_name: &str, opts: SequenceCreateOptions) -> Result<Sequence, Error> {
    let lua = crate::lua_state();
    let b: LuaTable<_> = lua
        .get("box")
        .ok_or_else(|| ExecutionError("box == nil".into()))?;
    let b_schema: LuaTable<_> = b
        .get("schema")
        .ok_or_else(|| ExecutionError("box.schema == nil".into()))?;
    let b_s_seq: LuaTable<_> = b_schema
        .get("sequence")
        .ok_or_else(|| ExecutionError("box.schema.sequence == nil".into()))?;
    let seq_create: LuaFunction<_> = b_s_seq
        .get("create")
        .ok_or_else(|| ExecutionError("box.schema.sequence.create == nil".into()))?;
    let new_seq: LuaTable<_> = seq_create
        .call_with_args((seq_name, &opts))
        .map_err(LuaError::from)?;
    let seq_id: u32 = new_seq
        .get("id")
        .ok_or_else(|| ExecutionError(format!("box.sequence['{}'] == nil", seq_name).into()))?;
    Ok(Sequence::from_id(seq_id))
}
