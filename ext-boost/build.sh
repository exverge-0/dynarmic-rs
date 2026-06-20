#!/bin/sh
bcp \
    boost/icl/interval_set.hpp \
    boost/icl/interval_map.hpp \
    boost/container/static_vector.hpp \
    boost/container/stable_vector.hpp \
    boost/container/small_vector.hpp \
    boost/container/flat_set.hpp \
    boost/container/container_fwd.hpp \
    boost/variant/detail/apply_visitor_binary.hpp \
    boost/pool/pool_alloc.hpp \
    boost/unordered_map.hpp \
    boost/variant.hpp \
    --boost="$1" .
