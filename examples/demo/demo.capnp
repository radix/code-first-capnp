@0xfbb45a811fbe71f5;

struct Person {
  id @0 :UInt64;
  fullName @1 :Text;
  emailAddresses @2 :List(Text);
  age @3 :UInt16;
  isActive @4 :Bool;
  score @7 :Float64;
  tags @5 :List(Text);
  status @6 :Status;
}

struct Company {
  companyName @0 :Text;
  employees @1 :List(Person);
  foundedYear @2 :UInt32;
  isPublic @3 :Bool;
}

struct Status {
  union {
    active @0 :Void;
    inactive @1 :Void;
    pending @2 :Void;
    suspended @3 :Void;
  }
}

struct EnumWithData {
  union {
    myText :group {
      field0 @0 :Text;
    }
    image :group {
      url @1 :Text;
      caption @2 :Text;
    }
    video :group {
      field0 @3 :Text;
      field1 @4 :UInt32;
    }
  }
}

struct UserProfileV2 {
  username @0 :Text;
  email @2 :Text;
  active @4 :Bool;
  preferences @5 :List(Text);
  oldUserId @1 :UInt64;
  deprecatedTimestamp @3 :UInt64;
  removedMetadata @6 :Text;
}

struct EmptyStruct {
}
