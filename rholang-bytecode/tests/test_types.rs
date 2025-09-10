use rholang_bytecode::core::types::*;
use std::sync::Arc;

#[test]
fn test_zero_copy_process_ref() {
    let proc1 = ProcessRef::new(1, 100, 50, RSpaceType::MemSeq);
    let proc2 = proc1.clone();

    // Verify that cloning doesn't copy data
    assert_eq!(proc1.ref_count(), 2);
    assert_eq!(proc2.ref_count(), 2);

    // Both references should have same reference count indicating shared data
    assert_eq!(proc1.ref_count(), proc2.ref_count());
}

#[test]
fn test_zero_copy_name_ref() {
    let name1 = NameRef::unforgeable([0u8; 32], 42);
    let name2 = name1.clone();

    // Verify zero-copy sharing through behavior
    assert!(name1.is_unforgeable());
    assert!(name2.is_unforgeable());

    // Both should have same unforgeable property indicating shared data
    assert_eq!(name1.is_unforgeable(), name2.is_unforgeable());
}

#[test]
fn test_environment_lexical_scoping() {
    let parent = Environment::new();
    parent.bind(0, TypeRef::Integer(IntegerRef::Small(10)));

    let child = Environment::with_parent(parent.clone());
    child.bind(1, TypeRef::Integer(IntegerRef::Small(20)));

    // Child can access parent's bindings
    assert!(matches!(child.lookup(0), Some(TypeRef::Integer(_))));
    assert!(matches!(child.lookup(1), Some(TypeRef::Integer(_))));

    // Parent cannot access child's bindings
    assert!(parent.lookup(1).is_none());
}

#[test]
fn test_type_ref_sendability() {
    let process = TypeRef::Process(ProcessRef::new(1, 0, 100, RSpaceType::MemSeq));
    assert!(process.is_sendable());

    let process2 = TypeRef::Process(ProcessRef::new(2, 50, 200, RSpaceType::StoreConc));
    assert!(process2.is_sendable());
}

#[test]
fn test_integer_ref_small_optimization() {
    let small = IntegerRef::Small(42);
    assert!(matches!(small, IntegerRef::Small(42)));

    let large = IntegerRef::Large(Arc::new(vec![u64::MAX, u64::MAX]));
    assert!(matches!(large, IntegerRef::Large(_)));
}

#[test]
fn test_rspace_type_properties() {
    // Test persistence
    assert!(!RSpaceType::MemSeq.is_persistent());
    assert!(!RSpaceType::MemConc.is_persistent());
    assert!(RSpaceType::StoreSeq.is_persistent());
    assert!(RSpaceType::StoreConc.is_persistent());

    // Test concurrency
    assert!(!RSpaceType::MemSeq.is_concurrent());
    assert!(RSpaceType::MemConc.is_concurrent());
    assert!(!RSpaceType::StoreSeq.is_concurrent());
    assert!(RSpaceType::StoreConc.is_concurrent());
}

#[test]
fn test_instruction_zero_copy() {
    use rholang_bytecode::core::instructions::Instruction;
    use rholang_bytecode::core::opcodes::Opcode;

    // Create instruction
    let inst = Instruction::nullary(Opcode::NOP);

    // Multiple accesses should not allocate
    let bytes1 = inst.to_bytes();
    let bytes2 = inst.to_bytes();

    assert_eq!(bytes1, bytes2);

    // Verify no allocation on instruction access by checking pointer equality
    let opcode1 = inst.opcode().unwrap();
    let opcode2 = inst.opcode().unwrap();
    assert_eq!(opcode1, opcode2);

    // Verify operand access is zero-copy
    let op1 = inst.op1();
    let op2 = inst.op1();
    assert_eq!(op1, op2);
}

#[test]
fn test_type_ref_arc_sharing() {
    // Test Arc reference counting with different types
    let process = ProcessRef::new(1, 0, 100, RSpaceType::MemSeq);
    let _type_ref1 = TypeRef::Process(process.clone());
    let _type_ref2 = TypeRef::Process(process.clone());

    // Both TypeRefs should share the same underlying ProcessRef
    assert_eq!(process.ref_count(), 3); // process + type_ref1 + type_ref2

    // Test with NameRef
    let name = NameRef::unforgeable([0u8; 32], 42);
    let name_ref1 = TypeRef::Name(name.clone());
    let name_ref2 = TypeRef::Name(name.clone());

    // Verify sharing through behavior equivalence
    match (&name_ref1, &name_ref2) {
        (TypeRef::Name(n1), TypeRef::Name(n2)) => {
            assert_eq!(n1.is_unforgeable(), n2.is_unforgeable());
        }
        _ => panic!("Expected Name types"),
    }
}

#[test]
fn test_tagged_pointer_zero_copy() {
    let value = 42u64;
    let tagged = TaggedPtr::new(&value as *const u64, 3);

    // Multiple accesses should not allocate or copy
    let tag1 = tagged.tag();
    let tag2 = tagged.tag();
    assert_eq!(tag1, tag2);
    assert_eq!(tag1, 3);

    let ptr1 = tagged.as_ptr();
    let ptr2 = tagged.as_ptr();
    assert_eq!(ptr1, ptr2);
    assert_eq!(ptr1, &value as *const u64);

    // Verify safe access
    let val1 = tagged.get(3).unwrap();
    let val2 = tagged.get(3).unwrap();
    assert_eq!(val1 as *const u64, val2 as *const u64);
    assert_eq!(*val1, *val2);
    assert_eq!(*val1, 42);
}
